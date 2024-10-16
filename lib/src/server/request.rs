use std::{
    collections::HashMap,
    io::{Cursor, Seek, SeekFrom, Write},
    time::SystemTime,
};

use bold_proto::nfs4_proto::NfsStat4;
use tracing::{debug, error};
use vfs::VfsPath;

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
    pub filehandle_cache: Option<&'a mut HashMap<Vec<u8>, (SystemTime, Filehandle)>>,
    // local write cache for this client
    pub write_cache: Option<&'a mut Cursor<Vec<u8>>>,
    cache_ttl: u64,
}

impl<'a> NfsRequest<'a> {
    pub fn new(
        client_addr: String,
        cmanager: ClientManagerHandle,
        fmanager: FileManagerHandle,
        boot_time: u64,
        // cache ttl + filehandle
        filehandle_cache: Option<&'a mut HashMap<Vec<u8>, (SystemTime, Filehandle)>>,
        write_cache: Option<&'a mut Cursor<Vec<u8>>>,
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
            write_cache,
            // set filehandle cache ttl to 10 seconds
            cache_ttl: 10,
        }
    }

    pub fn client_addr(&self) -> &String {
        &self.client_addr
    }

    pub fn current_filehandle_id(&self) -> Option<Vec<u8>> {
        match self.filehandle {
            Some(ref fh) => Some(fh.id.clone()),
            None => None,
        }
    }

    pub fn current_filehandle(&self) -> Option<Filehandle> {
        // TODO handle None
        match self.filehandle {
            Some(ref fh) => Some(fh.clone()),
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
        let cache = self.filehandle_cache.as_mut().unwrap();
        let now: SystemTime = SystemTime::now();
        cache.insert(filehandle.id.clone(), (now, filehandle));
    }

    pub fn drop_filehandle_from_cache(&mut self, filehandle_id: Vec<u8>) {
        let cache = self.filehandle_cache.as_mut().unwrap();
        cache.remove(&filehandle_id);
    }

    pub fn get_filehandle_from_cache(&mut self, filehandle_id: Vec<u8>) -> Option<Filehandle> {
        let cache = self.filehandle_cache.as_ref().unwrap();
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

    pub fn write_cache_write(&mut self, offset: u64, data: &[u8], file: &VfsPath) -> u32 {
        debug!("writing to write cache");

        if let Some(filelike) = self.write_cache.as_mut() {
            // read in existing data if cache is empty
            if filelike.get_ref().len() == 0 {
                let mut file = file.open_file().unwrap();
                file.read_to_end(filelike.get_mut()).unwrap();
            }
            filelike.seek(SeekFrom::Start(offset)).unwrap();
            filelike.write_all(data).unwrap();
            return data.len() as u32;
        }
        0
    }

    pub fn write_cache_commit(&mut self, file: &VfsPath) {
        debug!("commit write cache");
        if let Some(filelike) = self.write_cache.as_mut() {
            let mut file = file.append_file().unwrap();
            let _ = file.seek(SeekFrom::Start(0));
            let content = filelike.get_ref();
            let count = file.write(content.as_slice()).unwrap() as u32;

            if count > 0 {
                file.flush().unwrap();
            }
            filelike.get_mut().clear();
        }
    }

    pub fn write_cache_reset(&mut self) {
        debug!("clear write cache");
        if let Some(filelike) = self.write_cache.as_mut() {
            filelike.get_mut().clear();
        }
    }

    pub async fn set_filehandle_id(
        &mut self,
        filehandle_id: Vec<u8>,
    ) -> Result<Filehandle, NfsStat4> {
        let res = self.fmanager.get_filehandle_for_id(filehandle_id).await;
        match res {
            Ok(ref fh) => {
                self.filehandle = Some(*fh.clone());
                Ok(*fh.clone())
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
