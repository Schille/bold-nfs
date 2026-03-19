use std::collections::HashMap;

use bold_proto::{
    nfs4_proto::xdr::{
        ACL4_SUPPORT_ALLOW_ACL, FH4_VOLATILE_ANY, MODE4_RGRP, MODE4_ROTH, MODE4_RUSR, OPEN4_SHARE_ACCESS_READ, OPEN4_SHARE_DENY_NONE, attrlist4, changeid4, fsid4, nfs_fh4, nfs_lease4, nfsstat4, utf8str_cs
    },
    nfs4_utils::{FileAttr, FileAttrValue},
};

mod filehandle;
use bytes::Bytes;
pub use filehandle::{FilehandleDb, FilehandleVfs};
use futures::{AsyncReadExt, AsyncWriteExt};
pub use handle::{FileManagerHandle, WriteCacheHandle};
mod caching;
mod handle;
mod locking;

use handle::FileManagerMessage;
use locking::{LockingState, LockingStateDb};
use tracing::{debug, error};
use vfs::async_vfs::AsyncVfsPath;

#[derive(Debug)]
pub struct FileManager {
    pub root: AsyncVfsPath,
    pub lease_time: nfs_lease4,
    pub hard_link_support: bool,
    pub symlink_support: bool,
    pub unique_handles: bool,
    pub fsid: u64,
    // database for all managed filehandles
    pub fhdb: FilehandleDb,
    // this field trackes a sequence number for filehandles
    pub next_fh_id: u128,
    // database for all managed locking states
    pub lockdb: LockingStateDb,
    // this field trackes a sequence number for stateids
    pub next_stateid_id: u64,
    pub boot_time: u64,
    pub cachedb: HashMap<nfs_fh4<Bytes>, WriteCacheHandle>,
}

impl FileManager {
    pub async fn new(root: AsyncVfsPath, fsid: Option<u64>) -> Self {
        let fsid = fsid.unwrap_or(152);
        let boot_time = std::time::UNIX_EPOCH.elapsed().unwrap().as_secs();
        let mut fmanager = FileManager {
            root: root.clone(),
            // lease time in seconds
            lease_time: nfs_lease4(60),
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
        fmanager.root_fh().await;
        fmanager
    }

    // actor main message handler for FileManager
    // fn handle_message(&mut self, msg: FileManagerMessage) {
    //     match msg {
    //         FileManagerMessage::GetRootFilehandle(req) => {
    //             let fh_wo_locks = self.root_fh();
    //             let fh = self.attach_locks(fh_wo_locks);
    //             req.respond_to.send(fh).unwrap();
    //         }
    //         FileManagerMessage::GetFilehandle(req) => {
    //             if req.filehandle.is_some() {
    //                 let fh = self.get_filehandle_by_id(&req.filehandle.unwrap());
    //                 match fh {
    //                     Some(fh_wo_locks) => {
    //                         let fh = self.attach_locks(fh_wo_locks);
    //                         req.respond_to.send(Some(fh)).unwrap();
    //                     }
    //                     None => {
    //                         debug!("Filehandle not found");
    //                         req.respond_to.send(None).unwrap();
    //                     }
    //                 }
    //             } else if req.path.is_some() {
    //                 let path = self.root.join(req.path.unwrap()).unwrap();
    //                 // check if file exists
    //                 if path.exists().unwrap() {
    //                     let fh_wo_locks = self.get_filehandle(&path);
    //                     let fh = self.attach_locks(fh_wo_locks);
    //                     req.respond_to.send(Some(fh)).unwrap();
    //                 } else {
    //                     debug!("File not found {:?}", path);
    //                     req.respond_to.send(None).unwrap();
    //                 }
    //             } else {
    //                 let fh_wo_locks = self.root_fh();
    //                 let fh = self.attach_locks(fh_wo_locks);
    //                 req.respond_to.send(Some(fh)).unwrap();
    //             }
    //         }
    //         FileManagerMessage::GetFilehandleAttrs(req) => {
    //             req.respond_to
    //                 .send(self.filehandle_attrs(&req.attrs_request, &req.filehandle_id))
    //                 .unwrap();
    //         }
    //         FileManagerMessage::CreateFile(req) => {
    //             let fh = self.create_file(&req.path);
    //             if let Some(mut fh) = fh {
    //                 let stateid = self.get_new_lockingstate_id();
    //                 let lock = LockingState::new_shared_reservation(
    //                     fh.id.clone(),
    //                     stateid,
    //                     req.client_id,
    //                     req.owner,
    //                     req.share_access,
    //                     req.share_deny,
    //                 );
    //                 // add this new locking state to the db
    //                 self.lockdb.insert(lock.clone());
    //                 fh.locks = vec![lock];
    //                 req.respond_to.send(Some(fh)).unwrap();
    //             } else {
    //                 req.respond_to.send(None).unwrap();
    //             }
    //         }
    //         FileManagerMessage::LockFile() => todo!(),
    //         FileManagerMessage::CloseFile() => todo!(),
    //         FileManagerMessage::RemoveFile(req) => {
    //             let filehandle = self.get_filehandle_by_path(&req.path.as_str().to_string());
    //             let mut parent_path = req.path.parent().as_str().to_string();
    //             match filehandle {
    //                 Some(filehandle) => {
    //                     // TODO check locks
    //                     if req.path.is_dir().unwrap() {
    //                         let _ = req.path.read_dir();
    //                     } else {
    //                         let _ = req.path.remove_file();
    //                     }
    //                     self.fhdb.remove_by_id(&filehandle.id);
    //                 }
    //                 None => {
    //                     if req.path.is_dir().unwrap() {
    //                         let _ = req.path.read_dir();
    //                     } else {
    //                         let _ = req.path.remove_file();
    //                     }
    //                 }
    //             }

    //             if parent_path.is_empty() {
    //                 // this is root
    //                 parent_path = "/".to_string();
    //             }

    //             let parent_filehandle = self.get_filehandle_by_path(&parent_path).unwrap();
    //             // TODO: check locks
    //             self.touch_filehandle(parent_filehandle);
    //             req.respond_to.send(()).unwrap()
    //         }
    //         FileManagerMessage::TouchFile(req) => {
    //             let filehandle = self.get_filehandle_by_id(&req.id);
    //             match filehandle {
    //                 Some(filehandle) => {
    //                     // TODO: check locks
    //                     self.touch_filehandle(filehandle);
    //                 }
    //                 None => {
    //                     // we don't do nothing here
    //                 }
    //             }
    //         }
    //         FileManagerMessage::GetWriteCacheHandle(req) => {
    //             let handle = self.get_cache_handle(req.filehandle, req.filemanager);
    //             req.respond_to.send(handle).unwrap();
    //         }
    //         FileManagerMessage::DropWriteCacheHandle(req) => {
    //             self.drop_cache_handle(&req.filehandle_id);
    //         }
    //         FileManagerMessage::UpdateFilehandle(req) => {
    //             self.update_filehandle(req);
    //         }
    //     }
    // }

    fn touch_filehandle(&mut self, filehandle: FilehandleVfs) {
        // create a new filehandle with refreshed attributes
        let fh = FilehandleVfs::new(
            filehandle.file.clone(),
            filehandle.id.clone(),
            filehandle.version,
        );
        self.fhdb.remove_by_id(&filehandle.id);
        debug!("Touching filehandle: {:?}", fh);
        // and replace the old one
        self.fhdb.insert(fh);
    }

    fn update_filehandle(&mut self, filehandle: FilehandleVfs) {
        debug!("Updateing filehandle: {:?}", &filehandle);
        self.fhdb.remove_by_id(&filehandle.id);
        // and replace the old one
        self.fhdb.insert(filehandle);
    }

    pub async fn create_file(
        &mut self,
        request_file: &AsyncVfsPath,
        client_id: u64,
        owner: Bytes,
        share_access: u32,
        share_deny: u32,
    ) -> Option<FilehandleVfs> {
        let newfile = match request_file.create_file().await {
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
        let mut fh = self.get_filehandle(newfile).await;
        let mut path = newfile.parent().as_str().to_string();
        if path.is_empty() {
            // this is root
            path = "/".to_string();
        }
        // TODO: check locks
        let parent_filehandle = self.get_filehandle_by_path(&path).unwrap();
        self.touch_filehandle(parent_filehandle);

        let stateid = self.get_new_lockingstate_id();
        let lock = LockingState::new_shared_reservation(
            fh.id.clone(),
            stateid,
            client_id,
            owner,
            share_access,
            share_deny,
        );
        // add this new locking state to the db
        self.lockdb.insert(lock.clone());
        fh.locks = vec![lock];
        self.touch_filehandle(fh.clone());
        Some(fh)
    }

    pub async fn remove_file(&mut self, filehandle: FilehandleVfs) {
        // TODO: handle file already removed
        let mut parent_path = filehandle.file.parent().as_str().to_string();
        // TODO check locks
        if filehandle.file.is_dir().await.unwrap() {
            let _ = filehandle.file.read_dir();
        } else {
            let _ = filehandle.file.remove_file();
        }
        self.fhdb.remove_by_id(&filehandle.id);

        if parent_path.is_empty() {
            // this is root
            parent_path = "/".to_string();
        }
        debug!("parent path {:?}", parent_path);
        let parent_filehandle = self.get_filehandle_by_path(&parent_path).unwrap();
        // TODO: check locks
        self.touch_filehandle(parent_filehandle);
    }

    fn get_new_lockingstate_id(&mut self) -> [u8; 12] {
        // create a new unique lockingstate id
        let mut id = vec![0_u8, 0_u8, 0_u8, 0_u8];
        id.extend(self.next_stateid_id.to_be_bytes().to_vec());
        self.next_stateid_id += 1;
        id.try_into().unwrap()
    }

    fn get_filehandle_id(&mut self, file: &AsyncVfsPath) -> nfs_fh4<Bytes> {
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

        // https://tools.ietf.org/html/rfc7530#section-4.2.3
        // this implements a "Volatile Filehandle"
        let mut id = vec![128_u8];
        id.extend(self.boot_time.to_be_bytes().to_vec());
        id.extend(self.next_fh_id.to_be_bytes().to_vec());
        id.extend(vec![1_u8]);

        debug!("created new filehandle id: {:?}", id);
        self.next_fh_id += 1;
        nfs_fh4(Bytes::from_iter(id))
    }

    async fn get_filehandle_by_id(&mut self, id: &nfs_fh4<Bytes>) -> Option<FilehandleVfs> {
        let fh = self.fhdb.get_by_id(id);
        if let Some(fh) = fh {
            if fh.file.exists().await.unwrap() {
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

    pub fn get_filehandle_by_path(&self, path: &String) -> Option<FilehandleVfs> {
        debug!("get_filehandle_by_path: {}", path);
        self.fhdb.get_by_path(path).cloned()
    }

    pub async fn get_filehandle(&mut self, file: &AsyncVfsPath) -> FilehandleVfs {
        let id = self.get_filehandle_id(file);
        match self.get_filehandle_by_id(&id).await {
            Some(fh) => fh.clone(),
            None => {
                let fh = FilehandleVfs::new(file.clone(), id, 0);
                debug!("Storing new filehandle: {:?}", fh);
                self.fhdb.insert(fh.clone());
                fh
            }
        }
    }

    pub async fn root_fh(&mut self) -> FilehandleVfs {
        self.get_filehandle(&self.root.clone()).await
    }

    pub fn create_shared_reservation(&mut self, filehandle: &mut FilehandleVfs, client_id: u64, owner: Bytes) {
        let stateid = self.get_new_lockingstate_id();
        let lock = LockingState::new_shared_reservation(
            filehandle.id.clone(),
            stateid,
            client_id,
            owner,
            OPEN4_SHARE_ACCESS_READ,
            OPEN4_SHARE_DENY_NONE,
        );
        // add this new locking state to the db
        self.lockdb.insert(lock.clone());
        filehandle.locks = vec![lock];
    }

    pub fn attach_locks(&self, mut filehandle: FilehandleVfs) -> FilehandleVfs {
        let locks = self.lockdb.get_by_filehandle_id(&filehandle.id);
        filehandle.locks = locks.into_iter().cloned().collect();
        filehandle
    }

    pub async fn get_cache_handle(&self, filehandle: &FilehandleVfs) -> WriteCacheHandle {
        WriteCacheHandle::new(filehandle.clone()).await
    }

    pub async fn drop_cache_handle(&mut self, filehandle_id: &nfs_fh4<Bytes>) {
        if self.cachedb.contains_key(filehandle_id) {
            self.cachedb.remove(filehandle_id);
        }
        let filehandle = self.get_filehandle_by_id(filehandle_id).await;
        if let Some(filehandle) = filehandle {
            self.update_filehandle(filehandle);
        }
    }

    pub async fn set_attr(
        &self,
        filehandle: &FilehandleVfs,
        attr_vals: &Vec<FileAttrValue>,
    ) -> Vec<FileAttr> {
        let mut attrsset = Vec::new();
        for attr in attr_vals.iter() {
            match attr {
                FileAttrValue::Size(args) => {
                    debug!("Set size to: {:?}", args);
                    let mut buf = vec![0_u8; *args as usize];
                    let mut file = filehandle.file.open_file().await.unwrap();
                    //let _ = file.rewind();
                    file.read_exact(&mut buf).await.unwrap();

                    let mut file = filehandle.file.append_file().await.unwrap();
                    //let _ = file.rewind();
                    file.write_all(&buf).await.unwrap();
                    file.flush().await.unwrap();
                    attrsset.push(FileAttr::Size);
                }
                _ => {
                    debug!("Not supported set attr requested for: {:?}", attr);
                }
            }
        }
        attrsset
    }

    pub async fn filehandle_attrs(
        &self,
        attr_request: Vec<FileAttr>,
        filehandle: &FilehandleVfs,
    ) -> Option<(Vec<FileAttr>, Vec<FileAttrValue>)> {
        let mut answer_attrs = Vec::new();
        let mut attrs = Vec::new();

        for fileattr in attr_request {
            match fileattr {
                FileAttr::SupportedAttrs => {
                    attrs.push(FileAttrValue::SupportedAttrs(
                        self.attr_supported_attrs().into(),
                    ));
                    answer_attrs.push(FileAttr::SupportedAttrs);
                }
                FileAttr::Type => {
                    attrs.push(FileAttrValue::Type(filehandle.attr_type().await));
                    answer_attrs.push(FileAttr::Type);
                }
                FileAttr::LeaseTime => {
                    attrs.push(FileAttrValue::LeaseTime(self.attr_lease_time()));
                    answer_attrs.push(FileAttr::LeaseTime);
                }
                FileAttr::Change => {
                    attrs.push(FileAttrValue::Change(changeid4(
                        filehandle.attr_change(filehandle.version).await,
                    )));
                    answer_attrs.push(FileAttr::Change);
                }
                FileAttr::Size => {
                    attrs.push(FileAttrValue::Size(filehandle.attr_size().await));
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
                    attrs.push(FileAttrValue::Fsid(FilehandleVfs::attr_fsid(
                        self.fsid, self.fsid,
                    )));
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
                    attrs.push(FileAttrValue::Fileid(filehandle.attr_fileid()));
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
                    attrs.push(FileAttrValue::Owner(utf8str_cs(filehandle.attr_owner())));
                    answer_attrs.push(FileAttr::Owner);
                }
                FileAttr::OwnerGroup => {
                    attrs.push(FileAttrValue::OwnerGroup(utf8str_cs(
                        filehandle.attr_owner_group(),
                    )));
                    answer_attrs.push(FileAttr::OwnerGroup);
                }
                FileAttr::SpaceUsed => {
                    attrs.push(FileAttrValue::SpaceUsed(filehandle.attr_space_used().await));
                    answer_attrs.push(FileAttr::SpaceUsed);
                }
                FileAttr::TimeAccess => {
                    attrs.push(FileAttrValue::TimeAccess(filehandle.attr_time_access()));
                    answer_attrs.push(FileAttr::TimeAccess);
                }
                FileAttr::TimeMetadata => {
                    attrs.push(FileAttrValue::TimeMetadata(filehandle.attr_time_metadata()));
                    answer_attrs.push(FileAttr::TimeMetadata);
                }
                FileAttr::TimeModify => {
                    attrs.push(FileAttrValue::TimeModify(filehandle.attr_time_modify()));
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
        Some((answer_attrs, attrs))
    }

    pub fn attr_lease_time(&self) -> nfs_lease4 {
        self.lease_time.clone()
    }

    pub fn attr_rdattr_error(&self) -> nfsstat4 {
        // rdattr_error:
        // The server uses this to specify the behavior of the client when
        // reading attributes.  See Section 4 for additional description.
        nfsstat4::NFS4ERR_INVAL
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

// #[cfg(test)]
// mod tests {
//     use actix::{Actor, System};
//     use vfs::{AltrootFS, PhysicalFS, AsyncVfsPath};

//     use crate::server::{
//         clientmanager::{ClientManager, ClientManagerHandler},
//         NfsRequest,
//     };

//     use super::{FileManager, FileManagerHandler};

//     #[test]
//     fn set_current_filehandle() {
//         let root: AsyncVfsPath = AltrootFS::new(AsyncVfsPath::new(PhysicalFS::new(
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
