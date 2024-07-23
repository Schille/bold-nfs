pub mod clientmanager;
pub mod filemanager;
pub mod nfs40;
use std::{
    convert::Infallible,
    task::{Context, Poll},
};

use futures::future::{ready, Ready};
use tower::Service;
use tracing::{debug, instrument};
use vfs::VfsPath;

use crate::{
    bold::{MsgType, RpcCallMsg},
    proto::rpc_proto::{AcceptBody, AcceptedReply, CallBody, OpaqueAuth, ReplyBody, RpcReplyMsg},
};

pub trait NFSProtoImpl {
    fn minor_version(&self) -> u32;

    fn new(root: VfsPath) -> Self;

    fn hash(&self) -> u64;

    fn null(&self, _: &CallBody) -> ReplyBody;

    fn compound(&mut self, msg: &CallBody) -> ReplyBody;
}

#[derive(Debug, Clone)]
pub struct NFSService<Proto> {
    server: Proto,
}

impl<Proto> NFSService<Proto> {
    pub fn new(protocol: Proto) -> Self {
        NFSService { server: protocol }
    }
}

impl<Proto> Service<RpcCallMsg> for NFSService<Proto>
where
    Proto: NFSProtoImpl + Clone + Send + Sync + 'static,
{
    type Response = Box<RpcReplyMsg>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Infallible>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[instrument(name = "client", skip_all)]
    fn call(&mut self, req: RpcCallMsg) -> Self::Future {
        debug!("{:?}", req);

        match req.body {
            MsgType::Call(ref call_body) => {
                // TODO: check nfs protocol version
                let body = match call_body.proc {
                    0 => self.server.null(call_body),
                    1 => self.server.compound(call_body),
                    _ => {
                        todo!("Invalid procedure")
                    }
                };

                let resp = RpcReplyMsg {
                    xid: req.xid,
                    body: MsgType::Reply(body),
                };
                debug!("{:?}", resp);
                return ready(Ok(Box::new(resp)));
            }
            _ => {
                todo!("Invalid message type")
            }
        }
    }
}
