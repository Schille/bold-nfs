pub mod clientmanager;
pub mod filemanager;
pub mod nfs40;
pub mod operation;
pub mod request;
pub mod response;

use async_trait::async_trait;

use bytes::Bytes;
use request::NfsRequest;
use tracing::debug;

use bold_proto::{
    onc_rpc::{MessageType, ReplyBody, RpcMessage},
    CallBody,
};

#[async_trait]
pub trait NfsProtoImpl: Sync {
    fn minor_version(&self) -> u32;

    fn new() -> Self;

    fn hash(&self) -> u64;

    async fn null<'a>(
        &self,
        _: &CallBody<Bytes, Bytes>,
        mut request: NfsRequest<'a>,
    ) -> (NfsRequest<'a>, ReplyBody<Bytes, Bytes>);

    async fn compound<'a>(
        &self,
        msg: &CallBody<Bytes, Bytes>,
        mut request: NfsRequest<'a>,
    ) -> (NfsRequest<'a>, ReplyBody<Bytes, Bytes>);
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
        rpc_call_message: &Box<RpcMessage<Bytes, Bytes>>,
        request: NfsRequest<'_>,
    ) -> Box<RpcMessage<Bytes, Bytes>> {
        // debug!("{:?}", rpc_call_message);

        match rpc_call_message.call_body() {
            Some(call_body) => {
                // TODO: check nfs protocol version
                let (request, body) = match call_body.procedure() {
                    0 => self.server.null(call_body, request).await,
                    1 => self.server.compound(call_body, request).await,
                    _ => {
                        todo!("Invalid procedure")
                    }
                };

                // end request
                request.close().await;
                let rpc_reply_message =
                    RpcMessage::new(rpc_call_message.xid(), MessageType::Reply(body));
                // debug!("{:?}", rpc_reply_message);
                Box::new(rpc_reply_message)
            }
            _ => {
                todo!("Invalid message type")
            }
        }
    }
}
