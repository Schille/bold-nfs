use tokio::sync::{mpsc, oneshot};
use tracing::debug;
use vfs::VfsPath;

use bold_proto::nfs4_proto::{
    Attrlist4, FileAttr, FileAttrValue, NfsLease4, NfsStat4, ACL4_SUPPORT_ALLOW_ACL,
    FH4_VOLATILE_ANY, MODE4_RGRP, MODE4_ROTH, MODE4_RUSR,
};

use super::{
    caching::run_file_write_cache, caching::WriteCache, filehandle::Filehandle, run_file_manager,
    FileManager,
};
use crate::server::filemanager::NfsFh4;

pub enum FileManagerMessage {
    GetRootFilehandle(GetRootFilehandleRequest),
    GetFilehandle(GetFilehandleRequest),
    GetFilehandleAttrs(GetFilehandleAttrsRequest),
    CreateFile(CreateFileRequest),
    RemoveFile(RemoveFileRequest),
    TouchFile(TouchFileRequest),
    UpdateFilehandle(Filehandle),
    LockFile(),
    CloseFile(),
    GetWriteCacheHandle(WriteCacheHandleRequest),
    DropWriteCacheHandle(DropCacheHandleRequest),
}

pub struct GetRootFilehandleRequest {
    pub respond_to: oneshot::Sender<Filehandle>,
}

pub struct GetFilehandleRequest {
    pub path: Option<String>,
    pub filehandle: Option<NfsFh4>,
    pub respond_to: oneshot::Sender<Option<Filehandle>>,
}

pub struct GetFilehandleAttrsRequest {
    pub filehandle_id: NfsFh4,
    pub attrs_request: Vec<FileAttr>,
    pub respond_to: oneshot::Sender<Option<(Vec<FileAttr>, Vec<FileAttrValue>)>>,
}

pub struct CreateFileRequest {
    pub path: VfsPath,
    pub client_id: u64,
    pub owner: Vec<u8>,
    pub share_access: u32,
    pub share_deny: u32,
    pub verifier: Option<[u8; 8]>,
    pub respond_to: oneshot::Sender<Option<Filehandle>>,
}

pub struct RemoveFileRequest {
    pub path: VfsPath,
    pub respond_to: oneshot::Sender<()>,
}

pub struct TouchFileRequest {
    pub id: NfsFh4,
}

pub struct WriteCacheHandleRequest {
    pub filemanager: FileManagerHandle,
    pub filehandle: Filehandle,
    pub respond_to: oneshot::Sender<WriteCacheHandle>,
}

pub struct DropCacheHandleRequest {
    pub filehandle_id: NfsFh4,
}

#[derive(Debug, Clone)]
pub struct FileManagerError {
    pub nfs_error: NfsStat4,
}

#[derive(Debug, Clone)]
pub struct FileManagerHandle {
    sender: mpsc::Sender<FileManagerMessage>,
    lease_time: u32,
    hard_link_support: bool,
    symlink_support: bool,
    unique_handles: bool,
}

impl FileManagerHandle {
    pub fn new(root: VfsPath, fsid: Option<u64>) -> Self {
        let (sender, receiver) = mpsc::channel(16);
        let fmanager = FileManager::new(receiver, root, fsid);
        // start the filemanager actor
        tokio::spawn(run_file_manager(fmanager));

        Self {
            sender,
            lease_time: 60,
            hard_link_support: false,
            symlink_support: false,
            unique_handles: false,
        }
    }

    async fn send_filehandle_request(
        &self,
        path: Option<String>,
        filehandle: Option<NfsFh4>,
    ) -> Result<Filehandle, FileManagerError> {
        let filehandle_set = filehandle.is_some();
        let (tx, rx) = oneshot::channel();
        let req = GetFilehandleRequest {
            path: path.clone(),
            filehandle,
            respond_to: tx,
        };
        self.sender
            .send(FileManagerMessage::GetFilehandle(req))
            .await
            .unwrap();
        match rx.await {
            Ok(fh) => {
                if let Some(fh) = fh {
                    return Ok(fh);
                }
                if let Some(path) = path {
                    debug!("File not found: {:?}", path);
                    if !filehandle_set {
                        Err(FileManagerError {
                            nfs_error: NfsStat4::Nfs4errNoent,
                        })
                    } else {
                        Err(FileManagerError {
                            nfs_error: NfsStat4::Nfs4errStale,
                        })
                    }
                } else {
                    debug!("Filehandle not found");
                    // https://datatracker.ietf.org/doc/html/rfc7530#section-4.2.3
                    // If the server can definitively determine that a
                    // volatile filehandle refers to an object that has been removed, the
                    // server should return NFS4ERR_STALE to the client (as is the case for
                    // persistent filehandles)
                    Err(FileManagerError {
                        nfs_error: NfsStat4::Nfs4errStale,
                    })
                }
            }
            Err(_) => Err(FileManagerError {
                nfs_error: NfsStat4::Nfs4errServerfault,
            }),
        }
    }

    pub async fn get_root_filehandle(&self) -> Result<Filehandle, FileManagerError> {
        self.send_filehandle_request(None, None).await
    }

    pub async fn get_filehandle_for_id(&self, id: NfsFh4) -> Result<Filehandle, FileManagerError> {
        self.send_filehandle_request(None, Some(id)).await
    }

    pub async fn get_filehandle_for_path(
        &self,
        path: String,
    ) -> Result<Filehandle, FileManagerError> {
        self.send_filehandle_request(Some(path), None).await
    }

    pub async fn get_filehandle_attrs(
        &self,
        filehandle_id: NfsFh4,
        attrs_request: Vec<FileAttr>,
    ) -> Result<(Vec<FileAttr>, Vec<FileAttrValue>), FileManagerError> {
        let (tx, rx) = oneshot::channel();
        let req = GetFilehandleAttrsRequest {
            filehandle_id,
            attrs_request,
            respond_to: tx,
        };
        self.sender
            .send(FileManagerMessage::GetFilehandleAttrs(req))
            .await
            .unwrap();
        match rx.await {
            Ok(attrs) => {
                if let Some(attrs) = attrs {
                    return Ok(attrs);
                }
                Err(FileManagerError {
                    nfs_error: NfsStat4::Nfs4errBadhandle,
                })
            }
            Err(_) => Err(FileManagerError {
                nfs_error: NfsStat4::Nfs4errServerfault,
            }),
        }
    }

    pub async fn create_file(
        &self,
        path: VfsPath,
        client_id: u64,
        owner: Vec<u8>,
        access: u32,
        deny: u32,
        verifier: Option<[u8; 8]>,
    ) -> Result<Filehandle, FileManagerError> {
        let (tx, rx) = oneshot::channel();
        let req = CreateFileRequest {
            path,
            client_id,
            owner,
            share_access: access,
            share_deny: deny,
            verifier,
            respond_to: tx,
        };
        self.sender
            .send(FileManagerMessage::CreateFile(req))
            .await
            .unwrap();
        match rx.await {
            Ok(fh) => {
                if let Some(fh) = fh {
                    return Ok(fh);
                }
                Err(FileManagerError {
                    // TODO: check if this is the correct error
                    nfs_error: NfsStat4::Nfs4errBadhandle,
                })
            }
            Err(_) => Err(FileManagerError {
                nfs_error: NfsStat4::Nfs4errServerfault,
            }),
        }
    }

    pub async fn remove_file(&self, path: VfsPath) -> Result<(), FileManagerError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(FileManagerMessage::RemoveFile(RemoveFileRequest {
                path,
                respond_to: tx,
            }))
            .await
            .unwrap();
        match rx.await {
            Ok(_) => Ok(()),
            Err(_) => Err(FileManagerError {
                nfs_error: NfsStat4::Nfs4errServerfault,
            }),
        }
    }

    pub async fn touch_file(&self, id: NfsFh4) {
        self.sender
            .send(FileManagerMessage::TouchFile(TouchFileRequest { id }))
            .await
            .unwrap();
    }

    pub async fn update_filehandle(&self, filehandle: Filehandle) {
        self.sender
            .send(FileManagerMessage::UpdateFilehandle(filehandle))
            .await
            .unwrap();
    }

    pub async fn get_write_cache_handle(
        &self,
        filehandle: Filehandle,
    ) -> Result<WriteCacheHandle, FileManagerError> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(FileManagerMessage::GetWriteCacheHandle(
                WriteCacheHandleRequest {
                    filemanager: self.clone(),
                    filehandle,
                    respond_to: tx,
                },
            ))
            .await
            .unwrap();
        match rx.await {
            Ok(handle) => Ok(handle),
            Err(_) => Err(FileManagerError {
                nfs_error: NfsStat4::Nfs4errServerfault,
            }),
        }
    }

    pub async fn drop_write_cache_handle(&self, filehandle_id: NfsFh4) {
        self.sender
            .send(FileManagerMessage::DropWriteCacheHandle(
                DropCacheHandleRequest { filehandle_id },
            ))
            .await
            .unwrap();
    }

    pub fn filehandle_attrs(
        &mut self,
        attr_request: &Vec<FileAttr>,
        filehandle: &Filehandle,
    ) -> Option<(Attrlist4<FileAttr>, Attrlist4<FileAttrValue>)> {
        let mut answer_attrs = Attrlist4::<FileAttr>::new(None);
        let mut attrs = Attrlist4::<FileAttrValue>::new(None);

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
        Some((answer_attrs, attrs))
    }

    pub fn set_attr(
        &self,
        filehandle: &Filehandle,
        attr_vals: &Attrlist4<FileAttrValue>,
    ) -> Attrlist4<FileAttr> {
        let mut attrsset = Attrlist4::<FileAttr>::new(None);
        for attr in attr_vals.iter() {
            match attr {
                FileAttrValue::Size(args) => {
                    debug!("Set size to: {:?}", args);
                    let mut buf = vec![0_u8; *args as usize];
                    let mut file = filehandle.file.open_file().unwrap();
                    let _ = file.rewind();
                    file.read_exact(&mut buf).unwrap();

                    let mut file = filehandle.file.append_file().unwrap();
                    let _ = file.rewind();
                    file.write_all(&buf).unwrap();
                    file.flush().unwrap();
                    attrsset.push(FileAttr::Size);
                }
                _ => {
                    debug!("Not supported set attr requested for: {:?}", attr);
                }
            }
        }
        attrsset
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

    pub fn attr_supported_attrs(&self) -> Attrlist4<FileAttr> {
        // supported_attrs:
        // The bit vector that would retrieve all REQUIRED and RECOMMENDED
        // attributes that are supported for this object.  The scope of this
        //attribute applies to all objects with a matching fsid.
        Attrlist4::<FileAttr>::new(Some(vec![
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
        ]))
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

pub enum WriteCacheMessage {
    Write(WriteBytesRequest),
    Commit,
}

pub struct WriteBytesRequest {
    // seek offset
    pub offset: u64,
    // bytes to insert
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct WriteCacheHandle {
    sender: mpsc::Sender<WriteCacheMessage>,
}

impl WriteCacheHandle {
    pub fn new(filehandle: Filehandle, filemanager: FileManagerHandle) -> Self {
        let (sender, receiver) = mpsc::channel(16);
        let write_cache = WriteCache::new(receiver, filehandle, filemanager);
        // start the writecache actor
        tokio::spawn(run_file_write_cache(write_cache));

        Self { sender }
    }

    pub async fn write_bytes(&self, offset: u64, data: Vec<u8>) {
        self.sender
            .send(WriteCacheMessage::Write(WriteBytesRequest { offset, data }))
            .await
            .unwrap();
    }

    pub async fn commit(&self) {
        self.sender.send(WriteCacheMessage::Commit).await.unwrap();
    }
}
