use async_trait::async_trait;

use super::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};
use bold_proto::{nfs4_proto::*, rpc_proto::*};

mod op_access;
mod op_close;
mod op_getattr;
mod op_lookup;
mod op_open;
mod op_openconfirm;
mod op_putfh;
mod op_read;
mod op_readdir;
mod op_remove;
mod op_renew;
mod op_set_clientid;
mod op_set_clientid_confirm;
mod op_setattr;
mod op_write;

use super::NfsProtoImpl;
use tracing::error;

#[derive(Debug, Clone)]
pub struct NFS40Server;

impl NFS40Server {
    async fn put_root_filehandle(&self, mut request: NfsRequest) -> NfsOpResponse {
        match request.file_manager().get_root_filehandle().await {
            Ok(filehandle) => {
                request.set_filehandle_id(filehandle.id);
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

    fn operation_not_supported(&self, request: NfsRequest) -> NfsOpResponse {
        NfsOpResponse {
            request,
            result: None,
            status: NfsStat4::Nfs4errNotsupp,
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
        let mut last_status = NfsStat4::Nfs4Ok;
        let res = match &msg.args {
            Some(args) => {
                let mut resarray = Vec::with_capacity(args.argarray.len());
                // The server will process the COMPOUND procedure by evaluating each of
                // the operations within the COMPOUND procedure in order.
                for arg in &args.argarray {
                    let response = match arg {
                        // these should never be called
                        NfsArgOp::OpUndef0 | NfsArgOp::OpUndef1 | NfsArgOp::OpUndef2 => {
                            self.operation_not_supported(request)
                        }
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
                        NfsArgOp::Opsetattr(args) => args.execute(request).await,
                        NfsArgOp::Opremove(args) => args.execute(request).await,
                        NfsArgOp::Opwrite(args) => args.execute(request).await,

                        NfsArgOp::Opcommit(_) => self.operation_not_supported(request),
                        NfsArgOp::Opcreate(_) => self.operation_not_supported(request),
                        NfsArgOp::Opdelegpurge(_) => self.operation_not_supported(request),
                        NfsArgOp::Opdelegreturn(_) => self.operation_not_supported(request),

                        NfsArgOp::Oplink(_) => self.operation_not_supported(request),
                        NfsArgOp::Oplock(_) => self.operation_not_supported(request),
                        NfsArgOp::Oplockt(_) => self.operation_not_supported(request),
                        NfsArgOp::Oplocku(_) => self.operation_not_supported(request),

                        NfsArgOp::Oplookupp(_) => self.operation_not_supported(request),
                        NfsArgOp::Opnverify(_) => self.operation_not_supported(request),

                        NfsArgOp::Opopenattr(_) => self.operation_not_supported(request),

                        NfsArgOp::OpopenDowngrade(_) => self.operation_not_supported(request),

                        NfsArgOp::Opputpubfh(_) => self.operation_not_supported(request),

                        NfsArgOp::Opreadlink(_) => self.operation_not_supported(request),

                        NfsArgOp::Oprename(_) => self.operation_not_supported(request),

                        NfsArgOp::Oprestorefh(_) => self.operation_not_supported(request),
                        NfsArgOp::Opsavefh(_) => self.operation_not_supported(request),
                        NfsArgOp::OpSecinfo(_) => self.operation_not_supported(request),

                        NfsArgOp::Opverify(_) => self.operation_not_supported(request),

                        NfsArgOp::OpreleaseLockOwner(_) => self.operation_not_supported(request),
                    };
                    // match the result of the operation, pass on success, return on error
                    let res = response.result;
                    last_status = response.status;
                    if let Some(res) = res {
                        resarray.push(res);
                    } else {
                        request = response.request;
                        break;
                    }
                    match last_status {
                        NfsStat4::Nfs4Ok => {}
                        _ => {
                            return (
                                response.request,
                                ReplyBody::MsgAccepted(AcceptedReply {
                                    verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                    reply_data: AcceptBody::Success(Compound4res {
                                        status: last_status,
                                        tag: "".to_string(),
                                        resarray,
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
                    status: last_status,
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
