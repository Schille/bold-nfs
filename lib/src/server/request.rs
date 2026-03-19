use async_lock::RwLock;
use std::{collections::HashMap, sync::Arc};

use bold_proto::nfs4_proto::xdr::{nfs_fh4, nfsstat4};
use bytes::Bytes;
use tracing::error;

use super::{
    clientmanager::ClientManagerHandle,
    filemanager::{FileManager, FilehandleVfs, WriteCacheHandle},
};

#[derive(Debug)]
pub struct NfsRequest<'a> {
    client_addr: String,
    filehandle: Option<nfs_fh4<Bytes>>,
    // shared state for client manager between connections
    cmanager: ClientManagerHandle,
    // local filehandle manager
    // fmanager: FileManagerHandle,
    // time the server was booted
    pub boot_time: u64,
    // time the request was received
    pub request_time: u64,
    // filehandle db
    pub file_manager: Arc<RwLock<FileManager>>,
    // locally cached filehandles for this client
    pub writehandle_cache: Option<&'a mut HashMap<nfs_fh4<Bytes>, WriteCacheHandle>>,

    cache_ttl: u64,
}

impl<'a> NfsRequest<'a> {
    pub fn new(
        client_addr: String,
        cmanager: ClientManagerHandle,
        // fmanager: FileManagerHandle,
        boot_time: u64,
        // filehandle manager
        filehandle_manager: Arc<RwLock<FileManager>>,
        // cache ttl + filehandle
        writehandle_cache: Option<&'a mut HashMap<nfs_fh4<Bytes>, WriteCacheHandle>>,
    ) -> Self {
        let request_time = std::time::UNIX_EPOCH.elapsed().unwrap().as_secs();

        NfsRequest {
            client_addr,
            filehandle: None,
            cmanager,
            // fmanager,
            boot_time,
            request_time,
            writehandle_cache: writehandle_cache,
            file_manager: filehandle_manager,
            // set filehandle cache ttl to 10 seconds
            cache_ttl: 10,
        }
    }

    pub fn client_addr(&self) -> &String {
        &self.client_addr
    }

    pub fn current_filehandle_id(&self) -> Option<nfs_fh4<Bytes>> {
        match &self.filehandle {
            Some(fh) => Some(fh.clone()),
            None => None,
        }
    }

    pub async fn current_filehandle_vfs(&self) -> Option<FilehandleVfs> {
        // TODO handle None
        match self.filehandle {
            Some(ref fh) => {
                let rmanager = self.file_manager.read().await;
                let fh_vfs = rmanager.fhdb.get_by_id(&fh);
                if let Some(fh_vfs) = fh_vfs {
                    return Some(fh_vfs.clone());
                }
                None
            }
            None => None,
        }
    }

    pub fn client_manager(&self) -> ClientManagerHandle {
        self.cmanager.clone()
    }

    // pub fn file_manager(&self) -> FileManagerHandle {
    //     self.fmanager.clone()
    // }

    // pub fn set_filehandle(&mut self, filehandle: nfs_fh4) {
    //     self.filehandle = Some(filehandle);
    // }

    pub fn cache_writehandle(&mut self, fh: nfs_fh4<Bytes>, writecache: WriteCacheHandle) {
        let cache = self.writehandle_cache.as_mut();
        match cache {
            None => return,
            Some(cache) => {
                cache.insert(fh, writecache);
            }
        }
    }

    pub fn drop_filehandle_from_cache(&mut self, filehandle_id: &nfs_fh4<Bytes>) {
        let cache = self.writehandle_cache.as_mut();
        match cache {
            None => return,
            Some(cache) => {
                cache.remove(filehandle_id);
            }
        }
    }

    pub fn get_writehandle_from_cache(
        &mut self,
        filehandle_id: &nfs_fh4<Bytes>,
    ) -> Option<WriteCacheHandle> {
        // if no cache set, return None
        let cache = self.writehandle_cache.as_ref();
        match cache {
            None => None,
            Some(cache) => match cache.get(filehandle_id) {
                Some(fh) => Some(fh.clone()),
                None => None,
            },
        }
    }

    pub async fn set_filehandle(
        &mut self,
        filehandle_id: &nfs_fh4<Bytes>,
    ) -> Result<FilehandleVfs, nfsstat4> {
        let rmanager = self.file_manager.read().await;
        let fh: Option<&FilehandleVfs> = rmanager.fhdb.get_by_id(filehandle_id);
        match fh {
            Some(fh) => {
                self.filehandle = Some(fh.id.clone());
                Ok(fh.clone())
            }
            None => {
                error!("couldn't set filehandle");
                Err(nfsstat4::NFS4ERR_STALE)
            }
        }
    }

    pub fn unset_filehandle(&mut self) {
        self.filehandle = None;
    }

    // this is called when the request is done
    pub async fn close(&self) {
        // if let Some(fh) = self.filehandle.as_ref() {
        //     self.cmanager
        //         .set_current_filehandle(self.client_addr.clone(), fh.id.clone())
        //         .await;
        // }
    }
}
