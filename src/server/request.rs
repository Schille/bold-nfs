use super::{
    clientmanager::ClientManagerHandle,
    filemanager::{FileManagerHandle, Filehandle},
};

#[derive(Debug)]
pub struct NfsRequest {
    client_addr: String,
    filehandle_id: Option<Vec<u8>>,
    // shared state for client manager between connections
    cmanager: ClientManagerHandle,
    // local filehandle manager
    fmanager: FileManagerHandle,
}

impl NfsRequest {
    pub fn new(
        client_addr: String,
        cmanager: ClientManagerHandle,
        fmanager: FileManagerHandle,
    ) -> Self {
        NfsRequest {
            client_addr,
            filehandle_id: None,
            cmanager,
            fmanager,
        }
    }

    pub fn client_addr(&self) -> &String {
        &self.client_addr
    }

    pub fn current_filehandle_id(&self) -> Option<Vec<u8>> {
        self.filehandle_id.clone()
    }

    pub async fn current_filehandle(&self) -> Option<Box<Filehandle>> {
        match self.filehandle_id.as_ref() {
            Some(id) => {
                let fh = self.fmanager.get_filehandle_for_id(id.clone()).await;
                match fh {
                    Ok(fh) => Some(fh),
                    Err(_) => None,
                }
            }
            None => None,
        }
    }

    pub fn client_manager(&self) -> ClientManagerHandle {
        self.cmanager.clone()
    }

    pub fn file_manager(&self) -> FileManagerHandle {
        self.fmanager.clone()
    }

    pub fn set_filehandle_id(&mut self, filehandle_id: Vec<u8>) {
        self.filehandle_id = Some(filehandle_id);
    }

    // this is called when the request is done
    pub async fn close(&self) {
        if let Some(fh) = self.filehandle_id.as_ref() {
            self.cmanager
                .set_current_filehandle(self.client_addr.clone(), fh.clone())
                .await;
        }
    }
}
