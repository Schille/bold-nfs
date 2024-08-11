use std::{
    hash::{DefaultHasher, Hash, Hasher},
    iter,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::proto::nfs4_proto::{
    FileAttr, FileAttrValue, Fsid4, NfsFtype4, NfsLease4, NfsStat4, Nfstime4,
    ACL4_SUPPORT_ALLOW_ACL, FH4_PERSISTENT, MODE4_RGRP, MODE4_ROTH, MODE4_RUSR,
};
use actix::{Actor, Addr, Context, Handler, MailboxError, Message, MessageResult};
use multi_index_map::MultiIndexMap;
use vfs::VfsPath;

type FilehandleDb = MultiIndexFilehandleMap;

#[derive(MultiIndexMap, Debug, Clone)]
#[multi_index_derive(Debug, Clone)]
pub struct Filehandle {
    #[multi_index(hashed_unique)]
    pub id: Vec<u8>,
    pub file: VfsPath,
    // path:
    // the full path of the file including filename
    #[multi_index(hashed_unique)]
    pub path: String,
    // type:
    // Designates the type of an object in terms of one of a number of
    // special constants
    pub attr_type: NfsFtype4,
    // change:
    // A value created by the server that the client can use to determine if
    // file data, directory contents, or attributes of the object have been
    // modified.  The server MAY return the object's time_metadata attribute
    // for this attribute's value but only if the file system object cannot
    // be updated more frequently than the resolution of time_metadata.
    pub attr_change: u64,
    // size:
    // The size of the object in bytes.
    pub attr_size: u64,
    // fileid:
    // A number uniquely identifying the file within the file system.
    pub attr_fileid: u64,
    // fsid:
    // Unique file system identifier for the file system holding this
    // object.  The fsid attribute has major and minor components, each of
    // which are of data type uint64_t.
    pub attr_fsid: Fsid4,
    // mode:
    // The NFSv4.0 mode attribute is based on the UNIX mode bits.
    pub attr_mode: u32,
    // owner:
    // The string name of the owner of this object.
    pub attr_owner: String,
    // owner_group:
    // The string name of the group ownership of this object.
    pub attr_owner_group: String,
    // space_used:
    // Number of file system bytes allocated to this object.
    pub attr_space_used: u64,
    // time_access:
    // Represents the time of last access to the object by a READ operation
    // sent to the server.
    pub attr_time_access: Nfstime4,
    // time_metadata:
    // The time of last metadata modification of the object.
    pub attr_time_metadata: Nfstime4,
    // time_modified:
    // The time of last modification to the object.
    pub attr_time_modify: Nfstime4,
}

impl Filehandle {
    pub fn new(file: VfsPath, id: Vec<u8>, major: u64, minor: u64) -> Self {
        let init_time = Self::attr_time_access();
        let mut path = file.as_str().to_string();
        if path.is_empty() {
            path = "/".to_string();
        }
        Filehandle {
            id,
            path,
            attr_type: Self::attr_type(&file),
            attr_change: Self::attr_change(&file),
            attr_size: Self::attr_size(&file),
            attr_fileid: Self::attr_fileid(&file),
            attr_fsid: Self::attr_fsid(major, minor),
            attr_mode: Self::attr_mode(&file),
            attr_owner: Self::attr_owner(&file),
            attr_owner_group: Self::attr_owner_group(&file),
            attr_space_used: Self::attr_space_used(&file),
            attr_time_access: init_time,
            attr_time_metadata: init_time,
            attr_time_modify: init_time,
            file,
        }
    }

    fn attr_type(file: &VfsPath) -> NfsFtype4 {
        if file.is_dir().unwrap() {
            return NfsFtype4::Nf4dir;
        }
        if file.is_file().unwrap() {
            return NfsFtype4::Nf4reg;
        }
        NfsFtype4::Nf4Undef
    }

    fn attr_change(file: &VfsPath) -> u64 {
        let v = file.metadata().unwrap().modified.unwrap();
        u64::try_from(v.duration_since(UNIX_EPOCH).unwrap().as_secs()).unwrap()
    }

    fn attr_fileid(file: &VfsPath) -> u64 {
        let mut hasher = DefaultHasher::new();
        file.as_str().hash(&mut hasher);
        
        u64::try_from(hasher.finish()).unwrap()
    }

    fn attr_fsid(major: u64, minor: u64) -> Fsid4 {
        Fsid4 { major, minor }
    }

    fn attr_mode(_file: &VfsPath) -> u32 {
        MODE4_RUSR + MODE4_RGRP + MODE4_ROTH
    }

    fn attr_owner(_file: &VfsPath) -> String {
        "1000".to_string()
    }

    fn attr_owner_group(_file: &VfsPath) -> String {
        "1000".to_string()
    }

    fn attr_size(file: &VfsPath) -> u64 {
        u64::try_from(file.metadata().unwrap().len).unwrap()
    }

    fn attr_space_used(file: &VfsPath) -> u64 {
        u64::try_from(file.metadata().unwrap().len).unwrap()
    }

    fn attr_time_access() -> Nfstime4 {
        let since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        Nfstime4 {
            seconds: since_epoch.as_secs() as i64,
            nseconds: since_epoch.subsec_nanos(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileManager {
    pub root: VfsPath,
    pub lease_time: u32,
    pub hard_link_support: bool,
    pub symlink_support: bool,
    pub unique_handles: bool,
    pub fsid: u64,
    // database for all managed filehandles
    pub db: FilehandleDb,
}

impl Actor for FileManager {
    type Context = Context<Self>;
}

impl FileManager {
    pub fn new(root: VfsPath, fsid: Option<u64>) -> Self {
        let fsid = fsid.unwrap_or(152);
        
        FileManager {
            root: root.clone(),
            // lease time in seconds
            lease_time: 60,
            hard_link_support: false,
            symlink_support: false,
            unique_handles: true,
            fsid,
            db: FilehandleDb::default(),
        }
    }

    fn get_filehandle_id(&self, path: &VfsPath) -> Vec<u8> {
        let mut p: &str = path.as_str();

        if p.is_empty() {
            p = "/";
        }
        // TODO this does not work for long, just a dirty temporary solution
        let mut id: Vec<u8> = iter::repeat(0).take(128 - p.len()).collect();
        id.extend(p.as_bytes().to_vec());
        id
    }

    fn get_filehandle_by_id(&self, id: &Vec<u8>) -> Option<Filehandle> {
        self.db.get_by_id(id).cloned()
    }

    pub fn get_filehandle_by_path(&self, path: &String) -> Option<Filehandle> {
        print!("get_filehandle_by_path: {}", path);
        self.db.get_by_path(path).cloned()
    }

    pub fn get_filehandle(&mut self, file: &VfsPath) -> Filehandle {
        let id = self.get_filehandle_id(file);
        match self.get_filehandle_by_id(&id) {
            Some(fh) => fh.clone(),
            None => {
                let fh = Filehandle::new(file.clone(), id, self.fsid, self.fsid);
                self.db.insert(fh.clone());
                fh
            }
        }
    }

    pub fn root_fh(&mut self) -> Box<Filehandle> {
        Box::new(self.get_filehandle(&self.root.clone()))
    }

    pub fn filehandle_attrs(
        &self,
        attr_request: &Vec<FileAttr>,
        filehandle_id: &Vec<u8>,
    ) -> Box<(Vec<FileAttr>, Vec<FileAttrValue>)> {
        let mut answer_attrs = Vec::new();
        let mut attrs = Vec::new();
        let filehandle = self.get_filehandle_by_id(filehandle_id).unwrap();

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
        Box::new((answer_attrs, attrs))
    }

    // pub fn attr_filehandle(&self) -> &Vec<u8> {
    //     // filehandle:
    //     // The filehandle of this object (primarily for READDIR requests).
    //     &self.current_fh.as_ref().unwrap().id
    // }

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

    // pub fn attr_type(&self) -> NfsFtype4 {
    //     // type:
    //     // Designates the type of an object in terms of one of a number of
    //     // special constants
    //     self.current_fh.as_ref().unwrap().attr_type
    // }

    pub fn attr_expire_type(&self) -> u32 {
        // fh_expire_type:
        // The server uses this to specify filehandle expiration behavior to the
        // client.  See Section 4 for additional description.
        FH4_PERSISTENT
    }

    // pub fn attr_change(&self) -> u64 {
    //     // change:
    //     // A value created by the server that the client can use to determine if
    //     // file data, directory contents, or attributes of the object have been
    //     // modified.  The server MAY return the object's time_metadata attribute
    //     // for this attribute's value but only if the file system object cannot
    //     // be updated more frequently than the resolution of time_metadata.
    //     (self.current_fh.as_ref().unwrap().attr_time_modify.seconds * 1000000000
    //         + self.current_fh.as_ref().unwrap().attr_time_modify.nseconds as i64) as u64
    // }

    // pub fn attr_size(&self) -> u64 {
    //     // size:
    //     // The size of the object in bytes.
    //     self.current_fh.as_ref().unwrap().attr_size
    // }

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

    // pub fn attr_fsid(&self) -> Fsid4 {
    //     // fsid:
    //     // Unique file system identifier for the file system holding this
    //     // object.  The fsid attribute has major and minor components, each of
    //     // which are of data type uint64_t.
    //     self.current_fh.as_ref().unwrap().attr_fsid
    // }

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

    // pub fn attr_fileid(&self) -> u64 {
    //     // fileid:
    //     // A number uniquely identifying the file within the file system.
    //     self.current_fh.as_ref().unwrap().attr_fileid
    // }

    pub fn attr_mode(&self) -> u32 {
        // mode:
        // The NFSv4.0 mode attribute is based on the UNIX mode bits.
        MODE4_RUSR + MODE4_RGRP + MODE4_ROTH
    }

    pub fn attr_numlinks(&self) -> u32 {
        // numlinks:
        // Number of hard links to this object.
        2
    }

    // pub fn attr_owner(&self) -> &String {
    //     // owner:
    //     // The string name of the owner of this object.
    //     &self.current_fh.as_ref().unwrap().attr_owner
    // }

    // pub fn attr_owner_group(&self) -> String {
    //     // owner_group:
    //     // The string name of the group ownership of this object.
    //     self.current_fh.as_ref().unwrap().attr_owner_group.clone()
    // }

    // pub fn attr_space_used(&self) -> u64 {
    //     // space_used:
    //     // Number of file system bytes allocated to this object.
    //     self.current_fh.as_ref().unwrap().attr_space_used
    // }

    // pub fn attr_time_access(&self) -> Nfstime4 {
    //     // time_access:
    //     // Represents the time of last access to the object by a READ operation
    //     // sent to the server.
    //     self.current_fh.as_ref().unwrap().attr_time_access
    // }

    // pub fn attr_time_metadata(&self) -> Nfstime4 {
    //     // time_metadata:
    //     // The time of last metadata modification of the object.
    //     self.current_fh.as_ref().unwrap().attr_time_metadata
    // }

    // pub fn attr_time_modify(&self) -> Nfstime4 {
    //     // time_modified:
    //     // The time of last modification to the object.
    //     self.current_fh.as_ref().unwrap().attr_time_modify
    // }

    // pub fn attr_mounted_on_fileid(&self) -> u64 {
    //     // mounted_on_fileid:
    //     // Like fileid, but if the target filehandle is the root of a file
    //     // system, this attribute represents the fileid of the underlying
    //     // directory.
    //     self.current_fh.as_ref().unwrap().attr_fileid
    // }
}

#[derive(Message)]
#[rtype(result = "Box<Filehandle>")]
pub struct GetRootFilehandleRequest;

impl Handler<GetRootFilehandleRequest> for FileManager {
    type Result = MessageResult<GetRootFilehandleRequest>;

    fn handle(&mut self, _msg: GetRootFilehandleRequest, _ctx: &mut Context<Self>) -> Self::Result {
        MessageResult(self.root_fh())
    }
}

#[derive(Message)]
#[rtype(result = "Box<Filehandle>")]
pub struct GetFilehandleRequest {
    pub path: Option<String>,
    pub filehandle: Option<Vec<u8>>,
}

impl Handler<GetFilehandleRequest> for FileManager {
    type Result = MessageResult<GetFilehandleRequest>;

    fn handle(&mut self, msg: GetFilehandleRequest, _ctx: &mut Context<Self>) -> Self::Result {
        if msg.filehandle.is_some() {
            let fh = self.get_filehandle_by_id(&msg.filehandle.unwrap());
            match fh {
                Some(fh) => {
                    return MessageResult(Box::new(fh));
                }
                None => {
                    panic!("Filehandle not found");
                }
            }
        }
        if msg.path.is_some() {
            let path = self.root.join(msg.path.unwrap()).unwrap();
            let fh = self.get_filehandle(&path);
            MessageResult(Box::new(fh))
        } else {
            MessageResult(self.root_fh())
        }
    }
}

#[derive(Message)]
#[rtype(result = "Box<(Vec<FileAttr>, Vec<FileAttrValue>)>")]
pub struct GetFilehandleAttrsRequest {
    pub filehandle_id: Vec<u8>,
    pub attrs_request: Vec<FileAttr>,
}

impl Handler<GetFilehandleAttrsRequest> for FileManager {
    type Result = MessageResult<GetFilehandleAttrsRequest>;

    fn handle(&mut self, msg: GetFilehandleAttrsRequest, _ctx: &mut Context<Self>) -> Self::Result {
        MessageResult(self.filehandle_attrs(&msg.attrs_request, &msg.filehandle_id))
    }
}

#[derive(Debug, Clone)]
pub struct FileManagerHandler {
    pub fmanager: Addr<FileManager>,
}

impl FileManagerHandler {
    pub fn new(fmanager: Addr<FileManager>) -> Self {
        FileManagerHandler { fmanager }
    }

    async fn send_filehandle_request(
        &self,
        req: GetFilehandleRequest,
    ) -> Result<Box<Filehandle>, MailboxError> {
        let resp = self.fmanager.send(req).await;
        match resp {
            Ok(filehandle) => Ok(filehandle),
            Err(e) => Err(e),
        }
    }

    pub async fn get_root_filehandle(&self) -> Result<Box<Filehandle>, MailboxError> {
        let req = GetFilehandleRequest {
            path: None,
            filehandle: None,
        };
        self.send_filehandle_request(req).await
    }

    pub async fn get_filehandle_for_id(&self, id: Vec<u8>) -> Result<Box<Filehandle>, MailboxError> {
        let req = GetFilehandleRequest {
            path: None,
            filehandle: Some(id),
        };
        self.send_filehandle_request(req).await
    }

    pub async fn get_filehandle_for_path(
        &self,
        path: &String,
    ) -> Result<Box<Filehandle>, MailboxError> {
        let req = GetFilehandleRequest {
            path: Some(path.clone()),
            filehandle: None,
        };
        self.send_filehandle_request(req).await
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
