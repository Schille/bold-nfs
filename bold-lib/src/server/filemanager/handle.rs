use tokio::sync::{mpsc, oneshot};
use tracing::debug;
use vfs::VfsPath;

use crate::proto::nfs4_proto::{FileAttr, FileAttrValue, NfsStat4};

use super::{filehandle::Filehandle, run_file_manager, FileManager};

pub enum FileManagerMessage {
    GetRootFilehandle(GetRootFilehandleRequest),
    GetFilehandle(GetFilehandleRequest),
    GetFilehandleAttrs(GetFilehandleAttrsRequest),
    CreateFile(CreateFileRequest),
    RemoveFile(RemoveFileRequest),
    TouchFile(TouchFileRequest),
    LockFile(),
    CloseFile(),
}

pub struct GetRootFilehandleRequest {
    pub respond_to: oneshot::Sender<Box<Filehandle>>,
}

pub struct GetFilehandleRequest {
    pub path: Option<String>,
    pub filehandle: Option<Vec<u8>>,
    pub respond_to: oneshot::Sender<Option<Box<Filehandle>>>,
}

pub struct GetFilehandleAttrsRequest {
    pub filehandle_id: Vec<u8>,
    pub attrs_request: Vec<FileAttr>,
    pub respond_to: oneshot::Sender<Option<Box<(Vec<FileAttr>, Vec<FileAttrValue>)>>>,
}

pub struct CreateFileRequest {
    pub path: VfsPath,
    pub client_id: u64,
    pub owner: Vec<u8>,
    pub share_access: u32,
    pub share_deny: u32,
    verifier: Option<[u8; 8]>,
    pub respond_to: oneshot::Sender<Option<Box<Filehandle>>>,
}

pub struct RemoveFileRequest {
    pub path: VfsPath,
    pub respond_to: oneshot::Sender<()>,
}

pub struct TouchFileRequest {
    pub id: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct FileManagerError {
    pub nfs_error: NfsStat4,
}

#[derive(Debug, Clone)]
pub struct FileManagerHandle {
    sender: mpsc::Sender<FileManagerMessage>,
}

impl FileManagerHandle {
    pub fn new(root: VfsPath, fsid: Option<u64>) -> Self {
        let (sender, receiver) = mpsc::channel(16);
        let fmanager = FileManager::new(receiver, root, fsid);
        // start the filemanager actor
        tokio::spawn(run_file_manager(fmanager));

        Self { sender }
    }

    async fn send_filehandle_request(
        &self,
        path: Option<String>,
        filehandle: Option<Vec<u8>>,
    ) -> Result<Box<Filehandle>, FileManagerError> {
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

    pub async fn get_root_filehandle(&self) -> Result<Box<Filehandle>, FileManagerError> {
        self.send_filehandle_request(None, None).await
    }

    pub async fn get_filehandle_for_id(
        &self,
        id: Vec<u8>,
    ) -> Result<Box<Filehandle>, FileManagerError> {
        self.send_filehandle_request(None, Some(id)).await
    }

    pub async fn get_filehandle_for_path(
        &self,
        path: String,
    ) -> Result<Box<Filehandle>, FileManagerError> {
        self.send_filehandle_request(Some(path), None).await
    }

    pub async fn get_filehandle_attrs(
        &self,
        filehandle_id: Vec<u8>,
        attrs_request: Vec<FileAttr>,
    ) -> Result<Box<(Vec<FileAttr>, Vec<FileAttrValue>)>, FileManagerError> {
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
    ) -> Result<Box<Filehandle>, FileManagerError> {
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

    pub async fn touch_file(&self, id: Vec<u8>) {
        self.sender
            .send(FileManagerMessage::TouchFile(TouchFileRequest { id }))
            .await
            .unwrap();
    }
}
