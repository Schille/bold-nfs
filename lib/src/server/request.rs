use std::{collections::HashMap, time::SystemTime};

use bold_proto::nfs4_proto::{NfsFh4, NfsStat4};
use tracing::error;

use super::{
    clientmanager::ClientManagerHandle,
    filemanager::{FileManagerHandle, Filehandle},
};

#[derive(Debug)]
pub struct NfsRequest<'a> {
    client_addr: String,
    filehandle: Option<Filehandle>,
    // shared state for client manager between connections
    cmanager: ClientManagerHandle,
    // local filehandle manager
    fmanager: FileManagerHandle,
    // time the server was booted
    pub boot_time: u64,
    // time the request was received
    pub request_time: u64,
    // locally cached filehandles for this client
    pub filehandle_cache: Option<&'a mut HashMap<NfsFh4, (SystemTime, Filehandle)>>,
    cache_ttl: u64,
}

impl<'a> NfsRequest<'a> {
    pub fn new(
        client_addr: String,
        cmanager: ClientManagerHandle,
        fmanager: FileManagerHandle,
        boot_time: u64,
        // cache ttl + filehandle
        filehandle_cache: Option<&'a mut HashMap<NfsFh4, (SystemTime, Filehandle)>>,
    ) -> Self {
        let request_time = std::time::UNIX_EPOCH.elapsed().unwrap().as_secs();

        NfsRequest {
            client_addr,
            filehandle: None,
            cmanager,
            fmanager,
            boot_time,
            request_time,
            filehandle_cache,
            // set filehandle cache ttl to 10 seconds
            cache_ttl: 10,
        }
    }

    pub fn client_addr(&self) -> &String {
        &self.client_addr
    }

    pub fn current_filehandle_id(&self) -> Option<NfsFh4> {
        match self.filehandle {
            Some(ref fh) => Some(fh.id.clone()),
            None => None,
        }
    }

    pub fn current_filehandle(&self) -> Option<&Filehandle> {
        // TODO handle None
        match self.filehandle {
            Some(ref fh) => Some(fh),
            None => None,
        }
    }

    pub fn client_manager(&self) -> ClientManagerHandle {
        self.cmanager.clone()
    }

    pub fn file_manager(&self) -> FileManagerHandle {
        self.fmanager.clone()
    }

    pub fn set_filehandle(&mut self, filehandle: Filehandle) {
        self.filehandle = Some(filehandle);
    }

    pub fn cache_filehandle(&mut self, filehandle: Filehandle) {
        let cache = self.filehandle_cache.as_mut();
        match cache {
            None => return,
            Some(cache) => {
                let now: SystemTime = SystemTime::now();
                cache.insert(filehandle.id.clone(), (now, filehandle));
            }
        }
    }

    pub fn drop_filehandle_from_cache(&mut self, filehandle_id: NfsFh4) {
        let cache = self.filehandle_cache.as_mut();
        match cache {
            None => return,
            Some(cache) => {
                cache.remove(&filehandle_id);
            }
        }
    }

    pub fn get_filehandle_from_cache(&mut self, filehandle_id: NfsFh4) -> Option<Filehandle> {
        // if no cache set, return None
        let cache = self.filehandle_cache.as_ref();
        match cache {
            None => None,
            Some(cache) => {
                match cache.get(&filehandle_id) {
                    Some(fh) => {
                        let now: SystemTime = SystemTime::now();
                        let (time, filehandle) = fh;
                        // if cache is expired since 10 seconds, remove it
                        if now.duration_since(*time).unwrap().as_secs() > self.cache_ttl {
                            self.drop_filehandle_from_cache(filehandle.id.clone());
                            None
                        } else {
                            Some(filehandle.clone())
                        }
                    }
                    None => None,
                }
            }
        }
    }

    pub async fn set_filehandle_id(
        &mut self,
        filehandle_id: NfsFh4,
    ) -> Result<Filehandle, NfsStat4> {
        let res = self.fmanager.get_filehandle_for_id(filehandle_id).await;
        match res {
            Ok(ref fh) => {
                self.filehandle = Some(fh.clone());
                Ok(fh.clone())
            }
            Err(e) => {
                error!("couldn't set filehandle: {:?}", e);
                Err(NfsStat4::Nfs4errStale)
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
