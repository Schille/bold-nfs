pub mod clientmanager;
pub mod filemanager;
pub mod nfs40;
pub mod operation;
pub mod request;
pub mod response;

use async_trait::async_trait;

use request::NfsRequest;
use tracing::debug;

use bold_proto::rpc_proto::{CallBody, MsgType, ReplyBody, RpcCallMsg, RpcReplyMsg};

#[async_trait]
pub trait NfsProtoImpl: Sync {
    fn minor_version(&self) -> u32;

    fn new() -> Self;

    fn hash(&self) -> u64;

    async fn null<'a>(
        &self,
        _: CallBody,
        mut request: NfsRequest<'a>,
    ) -> (NfsRequest<'a>, ReplyBody);

    async fn compound<'a>(
        &self,
        msg: CallBody,
        mut request: NfsRequest<'a>,
    ) -> (NfsRequest<'a>, ReplyBody);
}

#[derive(Debug, Clone)]
pub struct NFSService<Proto> {
    server: Proto,
}

impl<Proto> NFSService<Proto>
where
    Proto: NfsProtoImpl,
{
    pub fn new(protocol: Proto) -> Self {
        NFSService { server: protocol }
    }

    pub async fn call(
        &self,
        rpc_call_message: RpcCallMsg,
        request: NfsRequest<'_>,
    ) -> Box<RpcReplyMsg> {
        debug!("{:?}", rpc_call_message);

        match rpc_call_message.body {
            MsgType::Call(call_body) => {
                // TODO: check nfs protocol version
                let (request, body) = match call_body.proc {
                    0 => self.server.null(call_body, request).await,
                    1 => self.server.compound(call_body, request).await,
                    _ => {
                        todo!("Invalid procedure")
                    }
                };

                // end request
                request.close().await;
                let rpc_reply_message = RpcReplyMsg {
                    xid: rpc_call_message.xid,
                    body: MsgType::Reply(body),
                };
                debug!("{:?}", rpc_reply_message);
                Box::new(rpc_reply_message)
            }
            _ => {
                todo!("Invalid message type")
            }
        }
    }
}
