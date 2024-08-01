pub mod clientmanager;
pub mod filemanager;
pub mod nfs40;
use std::{
    convert::Infallible,
    future,
    task::{Context, Poll},
};

use actix::Addr;
use async_trait::async_trait;
use clientmanager::ClientManager;
use filemanager::FileManager;
use futures::future::{ready, Ready};
use tower::Service;
use tracing::{debug, instrument};
use vfs::VfsPath;

use crate::{
    bold::{MsgType, RpcCallMsg},
    proto::rpc_proto::{
        self, AcceptBody, AcceptedReply, CallBody, OpaqueAuth, ReplyBody, RpcReplyMsg,
    },
};

#[async_trait]
pub trait NFSProtoImpl: Sync {
    fn minor_version(&self) -> u32;

    fn new(cmanager: Addr<ClientManager>, fmanager: Addr<FileManager>) -> Self;

    fn hash(&self) -> u64;

    fn null(&self, _: CallBody, client_addr: String) -> ReplyBody;

    async fn compound(&self, msg: CallBody, client_addr: String) -> ReplyBody;
}

#[derive(Debug, Clone)]
pub struct NFSService<Proto> {
    server: Proto,
    client_addr: String,
}

impl<Proto> NFSService<Proto>
where
    Proto: NFSProtoImpl,
{
    pub fn new(protocol: Proto, client_addr: String) -> Self {
        NFSService {
            server: protocol,
            client_addr: client_addr,
        }
    }

    pub async fn async_call(&self, req: RpcCallMsg, client_addr: String) -> Box<RpcReplyMsg> {
        debug!("{:?}", req);

        match req.body {
            MsgType::Call(call_body) => {
                // TODO: check nfs protocol version
                let body = match call_body.proc {
                    0 => self.server.null(call_body, client_addr),
                    1 => self.server.compound(call_body, client_addr).await,
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

// impl<Proto> NFSService<Proto>
// where
//     Proto: NFSProtoImpl + Clone + Send + Sync + 'static,
// {
//     type Response = Box<RpcReplyMsg>;
//     type Error = Infallible;
//     type Future = Ready<Result<Self::Response, Infallible>>;

//     fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }

//     #[instrument(name = "client", skip_all)]
//     async fn call(&mut self, req: RpcCallMsg) -> dyn future::Future<Output = Result<Self::Response, Self::Error>> {
//         debug!("{:?}", req);

//         match req.body {
//             MsgType::Call(ref call_body) => {
//                 // TODO: check nfs protocol version
//                 let body = match call_body.proc {
//                     0 => self.server.null(call_body),
//                     1 => self.server.compound(call_body).await,
//                     _ => {
//                         todo!("Invalid procedure")
//                     }
//                 };

//                 let resp = RpcReplyMsg {
//                     xid: req.xid,
//                     body: MsgType::Reply(body),
//                 };
//                 debug!("{:?}", resp);
//                 return ready(Ok(Box::new(resp)));
//             }
//             _ => {
//                 todo!("Invalid message type")
//             }
//         }
//     }
// }
