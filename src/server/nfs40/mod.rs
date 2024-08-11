use async_trait::async_trait;

use super::{request::NfsRequest, response::NfsOpResponse};
use crate::{
    proto::{nfs4_proto::*, rpc_proto::*},
    server::operation::NfsOperation,
};

mod op_access;
mod op_close;
mod op_getattr;
mod op_lookup;
mod op_open;
mod op_openconfirm;
mod op_putfh;
mod op_read;
mod op_readdir;
mod op_renew;
mod op_set_clientid;
mod op_set_clientid_confirm;

use super::NfsProtoImpl;
use tracing::error;

#[derive(Debug, Clone)]
pub struct NFS40Server;

impl NFS40Server {
    async fn put_root_filehandle(&self, mut request: NfsRequest) -> NfsOpResponse {
        match request.file_manager().get_root_filehandle().await {
            Ok(filehandle) => {
                request.set_filehandle_id(filehandle.id.clone());
                NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opputrootfh(PutRootFh4res {
                        status: NfsStat4::Nfs4Ok,
                    })),
                    status: NfsStat4::Nfs4Ok,
                }
            }
            Err(e) => {
                error!("Err {:?}", e);
                NfsOpResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errServerfault,
                }
            }
        }
    }

    fn get_current_filehandle(&self, request: NfsRequest) -> NfsOpResponse {
        let fh = request.current_filehandle_id();
        match fh {
            Some(filehandle_id) => NfsOpResponse {
                request,
                result: Some(NfsResOp4::Opgetfh(GetFh4res::Resok4(GetFh4resok {
                    object: filehandle_id,
                }))),
                status: NfsStat4::Nfs4Ok,
            },
            // current filehandle not set for client
            None => {
                error!("Filehandle not set");
                NfsOpResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errServerfault,
                }
            }
        }
    }
}

#[async_trait]
impl NfsProtoImpl for NFS40Server {
    fn new() -> Self {
        Self {}
    }

    fn hash(&self) -> u64 {
        0
    }

    async fn null(&self, _: CallBody, request: NfsRequest) -> (NfsRequest, ReplyBody) {
        (
            request,
            ReplyBody::MsgAccepted(AcceptedReply {
                verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                reply_data: AcceptBody::Success(Compound4res {
                    status: NfsStat4::Nfs4Ok,
                    tag: "".to_string(),
                    resarray: Vec::new(),
                }),
            }),
        )
    }

    async fn compound(&self, msg: CallBody, mut request: NfsRequest) -> (NfsRequest, ReplyBody) {
        let res = match &msg.args {
            Some(args) => {
                let mut resarray = Vec::with_capacity(args.argarray.len());
                // The server will process the COMPOUND procedure by evaluating each of
                // the operations within the COMPOUND procedure in order.
                for arg in &args.argarray {
                    let response = match arg {
                        // these should never be called
                        NfsArgOp::OpUndef0 | NfsArgOp::OpUndef1 | NfsArgOp::OpUndef2 => todo!(),
                        // these are actual operations
                        NfsArgOp::Opgetfh(_) => self.get_current_filehandle(request),
                        NfsArgOp::Opsetclientid(args) => args.execute(request).await,
                        NfsArgOp::OpAccess(args) => args.execute(request).await,
                        NfsArgOp::Opclose(args) => args.execute(request).await,
                        NfsArgOp::Opgetattr(args) => args.execute(request).await,
                        NfsArgOp::Oplookup(args) => args.execute(request).await,
                        NfsArgOp::Opopen(args) => args.execute(request).await,
                        NfsArgOp::OpopenConfirm(args) => args.execute(request).await,
                        NfsArgOp::Opputfh(args) => args.execute(request).await,
                        NfsArgOp::Opputrootfh(_) => self.put_root_filehandle(request).await,
                        NfsArgOp::Opread(args) => args.execute(request).await,
                        NfsArgOp::Opreaddir(args) => args.execute(request).await,
                        NfsArgOp::Oprenew(args) => args.execute(request).await,
                        NfsArgOp::OpsetclientidConfirm(args) => args.execute(request).await,

                        NfsArgOp::Opcommit(_) => todo!(),
                        NfsArgOp::Opcreate(_) => todo!(),
                        NfsArgOp::Opdelegpurge(_) => todo!(),
                        NfsArgOp::Opdelegreturn(_) => todo!(),

                        NfsArgOp::Oplink(_) => todo!(),
                        NfsArgOp::Oplock(_) => todo!(),
                        NfsArgOp::Oplockt(_) => todo!(),
                        NfsArgOp::Oplocku(_) => todo!(),

                        NfsArgOp::Oplookupp(_) => todo!(),
                        NfsArgOp::Opnverify(_) => todo!(),

                        NfsArgOp::Opopenattr(_) => todo!(),

                        NfsArgOp::OpopenDowngrade(_) => todo!(),

                        NfsArgOp::Opputpubfh(_) => todo!(),

                        NfsArgOp::Opreadlink(_) => todo!(),
                        NfsArgOp::Opremove(_) => todo!(),
                        NfsArgOp::Oprename(_) => todo!(),

                        NfsArgOp::Oprestorefh(_) => todo!(),
                        NfsArgOp::Opsavefh(_) => todo!(),
                        NfsArgOp::OpSecinfo(_) => todo!(),
                        NfsArgOp::Opsetattr(_) => todo!(),

                        NfsArgOp::Opverify(_) => todo!(),
                        NfsArgOp::Opwrite(_) => todo!(),
                        NfsArgOp::OpreleaseLockOwner(_) => todo!(),
                    };
                    // match the result of the operation, pass on success, return on error
                    match response.status {
                        NfsStat4::Nfs4Ok => resarray.push(response.result.unwrap()),
                        _ => {
                            return (
                                response.request,
                                ReplyBody::MsgAccepted(AcceptedReply {
                                    verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                    reply_data: AcceptBody::Success(Compound4res {
                                        status: response.status,
                                        tag: "".to_string(),
                                        resarray: Vec::new(),
                                    }),
                                }),
                            );
                        }
                    }
                    // pass on the request to the next operation
                    request = response.request;
                }
                resarray
            }
            None => Vec::new(),
        };

        (
            request,
            ReplyBody::MsgAccepted(AcceptedReply {
                verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                reply_data: AcceptBody::Success(Compound4res {
                    status: NfsStat4::Nfs4Ok,
                    tag: "".to_string(),
                    resarray: res,
                }),
            }),
        )
    }

    fn minor_version(&self) -> u32 {
        0
    }
}
