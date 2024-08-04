pub mod clientmanager;
pub mod filemanager;
pub mod nfs40;

use std::{borrow::BorrowMut, cell::RefCell, rc::Rc};

use actix::Addr;
use async_trait::async_trait;
use clientmanager::{ClientManager, SetCurrentFilehandleRequest};
use filemanager::{FileManager, FileManagerHandler, Filehandle, GetFilehandleRequest};

use tracing::debug;

use crate::{
    bold::{MsgType, RpcCallMsg},
    proto::rpc_proto::{CallBody, ReplyBody, RpcReplyMsg},
};

#[async_trait]
pub trait NFSProtoImpl: Sync {
    fn minor_version(&self) -> u32;

    fn new(cmanager: Addr<ClientManager>, fmanager: Addr<FileManager>) -> Self;

    fn hash(&self) -> u64;

    async fn null(&self, _: CallBody, request: NFSRequest) -> ReplyBody;

    async fn compound(&self, msg: CallBody, request: NFSRequest) -> ReplyBody;
}

#[derive(Debug, Clone)]
pub struct NFSService<Proto> {
    server: Proto,
}

impl<Proto> NFSService<Proto>
where
    Proto: NFSProtoImpl,
{
    pub fn new(protocol: Proto) -> Self {
        NFSService { server: protocol }
    }

    pub async fn call(&self, req: RpcCallMsg, request: NFSRequest) -> Box<RpcReplyMsg> {
        debug!("{:?}", req);

        match req.body {
            MsgType::Call(call_body) => {
                // TODO: check nfs protocol version
                let body = match call_body.proc {
                    0 => self.server.null(call_body, request).await,
                    1 => self.server.compound(call_body, request).await,
                    _ => {
                        todo!("Invalid procedure")
                    }
                };

                Box::new(RpcReplyMsg {
                    xid: req.xid,
                    body: MsgType::Reply(body),
                })
            }
            _ => {
                todo!("Invalid message type")
            }
        }
    }
}

#[derive(Debug)]
pub struct NFSRequest {
    client_addr: String,
    filehandle_id: Option<Vec<u8>>,
    filehandle_obj: Option<RefCell<Filehandle>>,
    // shared state for client manager between connections
    cmanager: Addr<ClientManager>,
    // local filehandle manager
    fmanager: FileManagerHandler,
}

impl NFSRequest {
    pub fn new(
        client_addr: String,
        cmanager: Addr<ClientManager>,
        fmanager: FileManagerHandler,
    ) -> Self {
        NFSRequest {
            client_addr,
            filehandle_id: None,
            filehandle_obj: None,
            cmanager,
            fmanager,
        }
    }

    pub fn client_addr(&self) -> &String {
        &self.client_addr
    }

    pub fn current_filehandle_id(&self) -> Option<Vec<u8>> {
        match self.filehandle_id {
            Some(fh) => Some(fh.clone()),
            None => None,
        }
    }

    pub async fn current_filehandle(&self) -> Option<Filehandle> {
        match self.filehandle_obj {
            Some(ref fh) => Some(fh.borrow().clone()),
            None => match self.filehandle_id {
                Some(ref id) => {
                    let fh = self.fmanager.get_filehandle_for_id(id).await;
                    match fh {
                        Ok(fh) => {
                            self.filehandle_obj.as_ref().replace(&RefCell::new(*fh.clone()));
                            Some(*fh)
                        }
                        Err(_) => None,
                    }
                }
                None => None,
            },
        }
    }

    pub fn client_manager(&self) -> Addr<ClientManager> {
        self.cmanager.clone()
    }

    pub fn file_manager(&self) -> FileManagerHandler {
        self.fmanager.clone()
    }

    pub fn set_filehandle(&mut self, filehandle: Vec<u8>) {
        self.filehandle = filehandle;
    }

    // this is called when the request is done
    pub async fn close(&self) {
        let _ = self
            .cmanager
            .send(SetCurrentFilehandleRequest {
                client_addr: self.client_addr.clone(),
                filehandle: self.filehandle.clone(),
            })
            .await;
    }
}
