use std::collections::HashMap;

use bold_proto::nfs4_proto::{
    FileAttr, FileAttrValue, NfsLease4, NfsStat4, ACL4_SUPPORT_ALLOW_ACL, FH4_VOLATILE_ANY,
    MODE4_RGRP, MODE4_ROTH, MODE4_RUSR,
};

mod filehandle;
pub use filehandle::Filehandle;
pub use handle::FileManagerHandle;
mod caching;
mod handle;
mod locking;

use filehandle::FilehandleDb;
use handle::{FileManagerMessage, WriteCacheHandle};
use locking::{LockingState, LockingStateDb};
use tokio::sync::mpsc;
use tracing::{debug, error};
use vfs::VfsPath;

#[derive(Debug)]
pub struct FileManager {
    pub root: VfsPath,
    pub lease_time: u32,
    pub hard_link_support: bool,
    pub symlink_support: bool,
    pub unique_handles: bool,
    pub fsid: u64,
    // database for all managed filehandles
    pub fhdb: FilehandleDb,
    // this field trackes a sequence number for filehandles
    pub next_fh_id: u64,
    // database for all managed locking states
    pub lockdb: LockingStateDb,
    // this field trackes a sequence number for stateids
    pub next_stateid_id: u64,
    pub boot_time: u64,
    // endpoint for incoming messages
    pub receiver: mpsc::Receiver<FileManagerMessage>,
    pub cachedb: HashMap<Vec<u8>, WriteCacheHandle>,
}

impl FileManager {
    pub fn new(
        receiver: mpsc::Receiver<FileManagerMessage>,
        root: VfsPath,
        fsid: Option<u64>,
    ) -> Self {
        let fsid = fsid.unwrap_or(152);
        let boot_time = std::time::UNIX_EPOCH.elapsed().unwrap().as_secs();
        let mut fmanager = FileManager {
            receiver,
            root: root.clone(),
            // lease time in seconds
            lease_time: 60,
            hard_link_support: false,
            symlink_support: false,
            unique_handles: false,
            boot_time,
            fsid,
            next_fh_id: 100,
            next_stateid_id: 100,
            fhdb: FilehandleDb::default(),
            lockdb: LockingStateDb::default(),
            cachedb: HashMap::new(),
        };
        // always have a root filehandle upon start
        fmanager.root_fh();
        fmanager
    }

    // actor main message handler for FileManager
    fn handle_message(&mut self, msg: FileManagerMessage) {
        match msg {
            FileManagerMessage::GetRootFilehandle(req) => {
                let fh_wo_locks = self.root_fh();
                let fh = self.attach_locks(fh_wo_locks);
                req.respond_to.send(Box::new(fh)).unwrap();
            }
            FileManagerMessage::GetFilehandle(req) => {
                if req.filehandle.is_some() {
                    let fh = self.get_filehandle_by_id(&req.filehandle.unwrap());
                    match fh {
                        Some(fh_wo_locks) => {
                            let fh = self.attach_locks(fh_wo_locks);
                            req.respond_to.send(Some(Box::new(fh))).unwrap();
                        }
                        None => {
                            debug!("Filehandle not found");
                            req.respond_to.send(None).unwrap();
                        }
                    }
                } else if req.path.is_some() {
                    let path = self.root.join(req.path.unwrap()).unwrap();
                    // check if file exists
                    if path.exists().unwrap() {
                        let fh_wo_locks = self.get_filehandle(&path);
                        let fh = self.attach_locks(fh_wo_locks);
                        req.respond_to.send(Some(Box::new(fh))).unwrap();
                    } else {
                        debug!("File not found {:?}", path);
                        req.respond_to.send(None).unwrap();
                    }
                } else {
                    let fh_wo_locks = self.root_fh();
                    let fh = self.attach_locks(fh_wo_locks);
                    req.respond_to.send(Some(Box::new(fh))).unwrap();
                }
            }
            FileManagerMessage::GetFilehandleAttrs(req) => {
                req.respond_to
                    .send(self.filehandle_attrs(&req.attrs_request, &req.filehandle_id))
                    .unwrap();
            }
            FileManagerMessage::CreateFile(req) => {
                let fh = self.create_file(&req.path);
                if let Some(mut fh) = fh {
                    let stateid = self.get_new_lockingstate_id();
                    let lock = LockingState::new_shared_reservation(
                        fh.id.clone(),
                        stateid,
                        req.client_id,
                        req.owner,
                        req.share_access,
                        req.share_deny,
                    );
                    // add this new locking state to the db
                    self.lockdb.insert(lock.clone());
                    fh.locks = vec![lock];
                    req.respond_to.send(Some(fh)).unwrap();
                } else {
                    req.respond_to.send(None).unwrap();
                }
            }
            FileManagerMessage::LockFile() => todo!(),
            FileManagerMessage::CloseFile() => todo!(),
            FileManagerMessage::RemoveFile(req) => {
                let filehandle = self.get_filehandle_by_path(&req.path.as_str().to_string());
                let mut parent_path = req.path.parent().as_str().to_string();
                match filehandle {
                    Some(filehandle) => {
                        // TODO check locks
                        if req.path.is_dir().unwrap() {
                            let _ = req.path.read_dir();
                        } else {
                            let _ = req.path.remove_file();
                        }
                        self.fhdb.remove_by_id(&filehandle.id);
                    }
                    None => {
                        if req.path.is_dir().unwrap() {
                            let _ = req.path.read_dir();
                        } else {
                            let _ = req.path.remove_file();
                        }
                    }
                }

                if parent_path.is_empty() {
                    // this is root
                    parent_path = "/".to_string();
                }

                let parent_filehandle = self.get_filehandle_by_path(&parent_path).unwrap();
                // TODO: check locks
                self.touch_filehandle(parent_filehandle);
                req.respond_to.send(()).unwrap()
            }
            FileManagerMessage::TouchFile(req) => {
                let filehandle = self.get_filehandle_by_id(&req.id);
                match filehandle {
                    Some(filehandle) => {
                        // TODO: check locks
                        self.touch_filehandle(filehandle);
                    }
                    None => {
                        // we don't do nothing here
                    }
                }
            }
            FileManagerMessage::GetWriteCacheHandle(req) => {
                let handle = self.get_cache_handle(req.filehandle, req.filemanager);
                req.respond_to.send(handle).unwrap();
            }
            FileManagerMessage::DropWriteCacheHandle(req) => {
                self.drop_cache_handle(req.filehandle_id);
            }
            FileManagerMessage::UpdateFilehandle(req) => {
                self.update_filehandle(req);
            }
        }
    }

    fn touch_filehandle(&mut self, filehandle: Filehandle) {
        // create a new filehandle with refreshed attributes
        let fh = Filehandle::new(
            filehandle.file.clone(),
            filehandle.id.clone(),
            self.fsid,
            self.fsid,
            filehandle.version,
        );
        self.fhdb.remove_by_id(&filehandle.id);
        debug!("Touching filehandle: {:?}", fh);
        // and replace the old one
        self.fhdb.insert(fh);
    }

    fn update_filehandle(&mut self, filehandle: Filehandle) {
        debug!("Updateing filehandle: {:?}", &filehandle);
        self.fhdb.remove_by_id(&filehandle.id);
        // and replace the old one
        self.fhdb.insert(filehandle);
    }

    fn create_file(&mut self, request_file: &VfsPath) -> Option<Box<Filehandle>> {
        let newfile = match request_file.create_file() {
            Ok(_) => {
                debug!("File created successfully");
                request_file
            }
            Err(e) => {
                error!("Error creating file {:?}", e);
                return None;
            }
        };

        // this filehandle is already added to the db
        let fh = self.get_filehandle(newfile);
        let mut path = newfile.parent().as_str().to_string();
        if path.is_empty() {
            // this is root
            path = "/".to_string();
        }
        // TODO: check locks
        let parent_filehandle = self.get_filehandle_by_path(&path).unwrap();
        self.touch_filehandle(parent_filehandle);

        Some(Box::new(fh))
    }

    fn get_new_lockingstate_id(&mut self) -> [u8; 12] {
        // create a new unique lockingstate id
        let mut id = vec![0_u8, 0_u8, 0_u8, 0_u8];
        id.extend(self.next_stateid_id.to_be_bytes().to_vec());
        self.next_stateid_id += 1;
        id.try_into().unwrap()
    }

    fn get_filehandle_id(&mut self, file: &VfsPath) -> Vec<u8> {
        // if there is already a filehandle for this path, return it
        let mut path = file.as_str().to_string();
        if path.is_empty() {
            // this is root
            path = "/".to_string();
        }
        let exists = self.get_filehandle_by_path(&path);
        if let Some(exists) = exists {
            return exists.id;
        }

        if path == "/" {
            // root gets a special filehandle that always constructs the same way
            return vec![128_u8];
        }
        // https://tools.ietf.org/html/rfc7530#section-4.2.3
        // this implements a "Volatile Filehandle"
        let mut id = vec![128_u8];
        id.extend(self.boot_time.to_be_bytes().to_vec());
        id.extend(self.next_fh_id.to_be_bytes().to_vec());
        id.extend(vec![1_u8]);

        debug!("created new filehandle id: {:?}", id);
        self.next_fh_id += 1;
        id
    }

    fn get_filehandle_by_id(&mut self, id: &Vec<u8>) -> Option<Filehandle> {
        let fh = self.fhdb.get_by_id(id);
        if let Some(fh) = fh {
            if fh.file.exists().unwrap() {
                debug!("Found filehandle: {:?}", fh);
                return Some(fh.clone());
            } else {
                // this filehandle is stale, remove it
                debug!("Removing stale filehandle: {:?}", fh);
                self.fhdb.remove_by_id(id);
            }
        }
        None
    }

    pub fn get_filehandle_by_path(&self, path: &String) -> Option<Filehandle> {
        debug!("get_filehandle_by_path: {}", path);
        self.fhdb.get_by_path(path).cloned()
    }

    pub fn get_filehandle(&mut self, file: &VfsPath) -> Filehandle {
        let id = self.get_filehandle_id(file);
        match self.get_filehandle_by_id(&id) {
            Some(fh) => fh.clone(),
            None => {
                let fh = Filehandle::new(file.clone(), id, self.fsid, self.fsid, 0);
                debug!("Storing new filehandle: {:?}", fh);
                self.fhdb.insert(fh.clone());
                fh
            }
        }
    }

    pub fn root_fh(&mut self) -> Filehandle {
        self.get_filehandle(&self.root.clone())
    }

    pub fn attach_locks(&self, mut filehandle: Filehandle) -> Filehandle {
        let locks = self.lockdb.get_by_filehandle_id(&filehandle.id);
        filehandle.locks = locks.into_iter().cloned().collect();
        filehandle
    }

    pub fn get_cache_handle(
        &mut self,
        mut filehandle: Filehandle,
        filemanager: FileManagerHandle,
    ) -> WriteCacheHandle {
        if self.cachedb.contains_key(&filehandle.id) {
            self.cachedb.get(&filehandle.id).unwrap().clone()
        } else {
            let handle = WriteCacheHandle::new(filehandle.clone(), filemanager);
            filehandle.write_cache = Some(handle.clone());
            self.cachedb.insert(filehandle.id.clone(), handle.clone());
            self.update_filehandle(filehandle);
            handle
        }
    }

    pub fn drop_cache_handle(&mut self, filehandle_id: Vec<u8>) {
        if self.cachedb.contains_key(&filehandle_id) {
            self.cachedb.remove(&filehandle_id);
        }
        let filehandle = self.get_filehandle_by_id(&filehandle_id);
        if let Some(mut filehandle) = filehandle {
            filehandle.write_cache = None;
            self.update_filehandle(filehandle);
        }
    }

    pub fn filehandle_attrs(
        &mut self,
        attr_request: &Vec<FileAttr>,
        filehandle_id: &Vec<u8>,
    ) -> Option<Box<(Vec<FileAttr>, Vec<FileAttrValue>)>> {
        let mut answer_attrs = Vec::new();
        let mut attrs = Vec::new();
        let fh = self.get_filehandle_by_id(filehandle_id);

        match fh {
            None => None,

            Some(filehandle) => {
                for fileattr in attr_request {
                    match fileattr {
                        FileAttr::SupportedAttrs => {
                            attrs.push(FileAttrValue::SupportedAttrs(self.attr_supported_attrs()));
                            answer_attrs.push(FileAttr::SupportedAttrs);
                        }
                        FileAttr::Type => {
                            attrs.push(FileAttrValue::Type(filehandle.attr_type));
                            answer_attrs.push(FileAttr::Type);
                        }
                        FileAttr::LeaseTime => {
                            attrs.push(FileAttrValue::LeaseTime(self.attr_lease_time()));
                            answer_attrs.push(FileAttr::LeaseTime);
                        }
                        FileAttr::Change => {
                            attrs.push(FileAttrValue::Change(filehandle.attr_change));
                            answer_attrs.push(FileAttr::Change);
                        }
                        FileAttr::Size => {
                            attrs.push(FileAttrValue::Size(filehandle.attr_size));
                            answer_attrs.push(FileAttr::Size);
                        }
                        FileAttr::LinkSupport => {
                            attrs.push(FileAttrValue::LinkSupport(self.attr_link_support()));
                            answer_attrs.push(FileAttr::LinkSupport);
                        }
                        FileAttr::SymlinkSupport => {
                            attrs.push(FileAttrValue::SymlinkSupport(self.attr_symlink_support()));
                            answer_attrs.push(FileAttr::SymlinkSupport);
                        }
                        FileAttr::NamedAttr => {
                            attrs.push(FileAttrValue::NamedAttr(self.attr_named_attr()));
                            answer_attrs.push(FileAttr::NamedAttr);
                        }
                        FileAttr::AclSupport => {
                            attrs.push(FileAttrValue::AclSupport(self.attr_acl_support()));
                            answer_attrs.push(FileAttr::AclSupport);
                        }
                        FileAttr::Fsid => {
                            attrs.push(FileAttrValue::Fsid(filehandle.attr_fsid));
                            answer_attrs.push(FileAttr::Fsid);
                        }
                        FileAttr::UniqueHandles => {
                            attrs.push(FileAttrValue::UniqueHandles(self.attr_unique_handles()));
                            answer_attrs.push(FileAttr::UniqueHandles);
                        }
                        FileAttr::FhExpireType => {
                            attrs.push(FileAttrValue::FhExpireType(self.attr_expire_type()));
                            answer_attrs.push(FileAttr::FhExpireType);
                        }
                        FileAttr::RdattrError => {
                            attrs.push(FileAttrValue::RdattrError(self.attr_rdattr_error()));
                            answer_attrs.push(FileAttr::RdattrError);
                        }
                        FileAttr::Fileid => {
                            attrs.push(FileAttrValue::Fileid(filehandle.attr_fileid));
                            answer_attrs.push(FileAttr::Fileid);
                        }
                        FileAttr::Mode => {
                            attrs.push(FileAttrValue::Mode(self.attr_mode()));
                            answer_attrs.push(FileAttr::Mode);
                        }
                        FileAttr::Numlinks => {
                            attrs.push(FileAttrValue::Numlinks(self.attr_numlinks()));
                            answer_attrs.push(FileAttr::Numlinks);
                        }
                        FileAttr::Owner => {
                            attrs.push(FileAttrValue::Owner(filehandle.attr_owner.clone()));
                            answer_attrs.push(FileAttr::Owner);
                        }
                        FileAttr::OwnerGroup => {
                            attrs.push(FileAttrValue::OwnerGroup(
                                filehandle.attr_owner_group.clone(),
                            ));
                            answer_attrs.push(FileAttr::OwnerGroup);
                        }
                        FileAttr::SpaceUsed => {
                            attrs.push(FileAttrValue::SpaceUsed(filehandle.attr_space_used));
                            answer_attrs.push(FileAttr::SpaceUsed);
                        }
                        FileAttr::TimeAccess => {
                            attrs.push(FileAttrValue::TimeAccess(filehandle.attr_time_access));
                            answer_attrs.push(FileAttr::TimeAccess);
                        }
                        FileAttr::TimeMetadata => {
                            attrs.push(FileAttrValue::TimeMetadata(filehandle.attr_time_metadata));
                            answer_attrs.push(FileAttr::TimeMetadata);
                        }
                        FileAttr::TimeModify => {
                            attrs.push(FileAttrValue::TimeModify(filehandle.attr_time_modify));
                            answer_attrs.push(FileAttr::TimeModify);
                        }
                        // FileAttr::MountedOnFileid => {
                        //     attrs.push(FileAttrValue::MountedOnFileid(
                        //         filehandle.attr_mounted_on_fileid,
                        //     ));
                        //     answer_attrs.push(FileAttr::MountedOnFileid);
                        // }
                        _ => {}
                    }
                }
                Some(Box::new((answer_attrs, attrs)))
            }
        }
    }

    pub fn attr_lease_time(&self) -> NfsLease4 {
        self.lease_time
    }

    pub fn attr_rdattr_error(&self) -> NfsStat4 {
        // rdattr_error:
        // The server uses this to specify the behavior of the client when
        // reading attributes.  See Section 4 for additional description.
        NfsStat4::Nfs4errInval
    }

    pub fn attr_supported_attrs(&self) -> Vec<FileAttr> {
        // supported_attrs:
        // The bit vector that would retrieve all REQUIRED and RECOMMENDED
        // attributes that are supported for this object.  The scope of this
        //attribute applies to all objects with a matching fsid.
        vec![
            FileAttr::SupportedAttrs,
            FileAttr::Type,
            FileAttr::FhExpireType,
            FileAttr::Change,
            FileAttr::Size,
            FileAttr::LinkSupport,
            FileAttr::SymlinkSupport,
            FileAttr::NamedAttr,
            FileAttr::Fsid,
            FileAttr::UniqueHandles,
            FileAttr::LeaseTime,
            FileAttr::RdattrError,
            FileAttr::Acl,
            FileAttr::AclSupport,
            FileAttr::Archive,
            // FileAttr::Cansettime,
            FileAttr::Filehandle,
            FileAttr::Fileid,
            FileAttr::Mode,
            FileAttr::Numlinks,
            FileAttr::Owner,
            FileAttr::OwnerGroup,
            FileAttr::SpaceUsed,
            FileAttr::TimeAccess,
            FileAttr::TimeMetadata,
            FileAttr::TimeModify,
            // FileAttr::MountedOnFileid,
        ]
    }

    pub fn attr_expire_type(&self) -> u32 {
        // fh_expire_type:
        // The server uses this to specify filehandle expiration behavior to the
        // client.  See Section 4 for additional description.
        FH4_VOLATILE_ANY
    }

    pub fn attr_link_support(&self) -> bool {
        // link_support:
        // TRUE, if the object's file system supports hard links.
        self.hard_link_support
    }

    pub fn attr_symlink_support(&self) -> bool {
        // symlink_support:
        // TRUE, if the object's file system supports symbolic links.
        self.symlink_support
    }

    pub fn attr_named_attr(&self) -> bool {
        // named_attr:
        // TRUE, if the object's has named attributes.  In other words, this
        // object has a non-empty named attribute directory.
        false
    }

    pub fn attr_unique_handles(&self) -> bool {
        // unique_handles:
        // TRUE, if two distinct filehandles are guaranteed to refer to two
        // different file system objects.
        self.unique_handles
    }

    pub fn attr_acl(&self) -> bool {
        // acl:
        // The NFSv4.0 ACL attribute contains an array of ACEs that are
        // associated with the file system object.  Although the client can read
        // and write the acl attribute, the server is responsible for using the
        // ACL to perform access control.  The client can use the OPEN or ACCESS
        // operations to check access without modifying or reading data or
        // metadata.
        false
    }

    pub fn attr_acl_support(&self) -> u32 {
        // acl_support:
        // TRUE, if the object's file system supports Access Control Lists.
        ACL4_SUPPORT_ALLOW_ACL
    }

    pub fn attr_archive(&self) -> bool {
        // archive:
        // TRUE, if the object's file system supports the archive attribute.
        false
    }

    pub fn attr_mode(&self) -> u32 {
        // mode:
        // The NFSv4.0 mode attribute is based on the UNIX mode bits.
        MODE4_RUSR + MODE4_RGRP + MODE4_ROTH
    }

    pub fn attr_numlinks(&self) -> u32 {
        // numlinks:
        // Number of hard links to this object.
        1
    }
}

// FileManager is run as with the actor pattern
// learn more: https://ryhl.io/blog/actors-with-tokio/
async fn run_file_manager(mut actor: FileManager) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg);
    }
}

// #[cfg(test)]
// mod tests {
//     use actix::{Actor, System};
//     use vfs::{AltrootFS, PhysicalFS, VfsPath};

//     use crate::server::{
//         clientmanager::{ClientManager, ClientManagerHandler},
//         NfsRequest,
//     };

//     use super::{FileManager, FileManagerHandler};

//     #[test]
//     fn set_current_filehandle() {
//         let root: VfsPath = AltrootFS::new(VfsPath::new(PhysicalFS::new(
//             std::env::current_dir().unwrap(),
//         )))
//         .into();

//         let sys = System::new();

//         sys.block_on(async move {
//             let client_manager_addr = ClientManager::new().start();
//             let file_manager_addr = FileManager::new(root, None).start();

//             let request = NfsRequest::new(
//                 "127.0.0.1:936".to_string(),
//                 ClientManagerHandler::new(client_manager_addr.clone()),
//                 FileManagerHandler::new(file_manager_addr.clone()),
//             );

//             let fh_id = vec![1u8, 2u8, 3u8];
//             request.set_filehandle_id(fh_id.clone());
//             assert_eq!(request.current_filehandle_id().unwrap(), fh_id);
//         });
//     }
// }
