use async_trait::async_trait;
use bold_proto::nfs4_gen::xdr::COMPOUND4args;
use bytes::Bytes;

use super::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};
use bold_proto::{nfs4_proto::*, rpc_proto::*};
use bold_proto::CallBody;
use bold_proto::nfs4_gen::xdr::nfs_argop4;

mod op_access;
mod op_close;
mod op_commit;
mod op_create;
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
    async fn put_root_filehandle<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        let file_manager = request.file_manager.clone();
        let manager = file_manager.read().unwrap();

        match manager.get_filehandle_by_path(&String::from("/")) {
            Some(filehandle) => {
                request.set_filehandle(filehandle.id);
                NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opputrootfh(PutRootFh4res {
                        status: NfsStat4::Nfs4Ok,
                    })),
                    status: NfsStat4::Nfs4Ok,
                }
            }
            None => NfsOpResponse {
                request,
                result: None,
                status: NfsStat4::Nfs4errServerfault,
            },
        }
    }

    fn get_current_filehandle<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
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

    fn operation_not_supported<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
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

    async fn null<'a>(&self, _: &CallBody<Bytes, Bytes>, request: NfsRequest<'a>) -> (NfsRequest<'a>, ReplyBody) {
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

    async fn compound<'a>(
        &self,
        msg: &CallBody<Bytes, Bytes>,
        mut request: NfsRequest<'a>,
    ) -> (NfsRequest<'a>, ReplyBody) {
        let mut last_status = NfsStat4::Nfs4Ok;
        let payload = msg.payload();
        let compound_args = COMPOUND4args::try_from(*payload);
        let res = match compound_args {
            Ok(args) => {
                let mut resarray = Vec::with_capacity(args.argarray.len());
                // The server will process the COMPOUND procedure by evaluating each of
                // the operations within the COMPOUND procedure in order.
                for arg in args.argarray {
                    let response = match arg {
                        // these should never be called
                        // nfs_argop4::OpUnef0 | nfs_argop4::OpUndef1 | nfs_argop4::OpUndef2 => {
                        //     self.operation_not_supported(request)
                        // }
                        // these are actual operations
                        nfs_argop4::OP_GETFH => self.get_current_filehandle(request),
                        nfs_argop4::OP_SETCLIENTID(args) => args.execute(request).await,
                        nfs_argop4::OpAccess(args) => args.execute(request).await,
                        nfs_argop4::Opclose(args) => args.execute(request).await,
                        nfs_argop4::Opgetattr(args) => args.execute(request).await,
                        nfs_argop4::Oplookup(args) => args.execute(request).await,
                        nfs_argop4::Opopen(args) => args.execute(request).await,
                        nfs_argop4::OpopenConfirm(args) => args.execute(request).await,
                        nfs_argop4::Opputfh(args) => args.execute(request).await,
                        nfs_argop4::Opputrootfh(_) => self.put_root_filehandle(request).await,
                        nfs_argop4::Opread(args) => args.execute(request).await,
                        nfs_argop4::Opreaddir(args) => args.execute(request).await,
                        nfs_argop4::Oprenew(args) => args.execute(request).await,
                        nfs_argop4::OpsetclientidConfirm(args) => args.execute(request).await,
                        nfs_argop4::Opsetattr(args) => args.execute(request).await,
                        nfs_argop4::Opremove(args) => args.execute(request).await,
                        nfs_argop4::Opwrite(args) => args.execute(request).await,

                        nfs_argop4::Opcommit(args) => args.execute(request).await,
                        nfs_argop4::Opcreate(args) => args.execute(request).await,

                        nfs_argop4::Opdelegpurge(_) => self.operation_not_supported(request),
                        nfs_argop4::Opdelegreturn(_) => self.operation_not_supported(request),

                        nfs_argop4::Oplink(_) => self.operation_not_supported(request),
                        nfs_argop4::Oplock(_) => self.operation_not_supported(request),
                        nfs_argop4::Oplockt(_) => self.operation_not_supported(request),
                        nfs_argop4::Oplocku(_) => self.operation_not_supported(request),

                        nfs_argop4::Oplookupp(_) => self.operation_not_supported(request),
                        nfs_argop4::Opnverify(_) => self.operation_not_supported(request),

                        nfs_argop4::Opopenattr(_) => self.operation_not_supported(request),

                        nfs_argop4::OpopenDowngrade(_) => self.operation_not_supported(request),

                        nfs_argop4::Opputpubfh(_) => self.operation_not_supported(request),

                        nfs_argop4::Opreadlink(_) => self.operation_not_supported(request),

                        nfs_argop4::Oprename(_) => self.operation_not_supported(request),

                        nfs_argop4::Oprestorefh(_) => self.operation_not_supported(request),
                        nfs_argop4::Opsavefh(_) => self.operation_not_supported(request),
                        nfs_argop4::OpSecinfo(_) => self.operation_not_supported(request),

                        nfs_argop4::Opverify(_) => self.operation_not_supported(request),

                        nfs_argop4::OpreleaseLockOwner(_) => self.operation_not_supported(request),
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
            _ => Vec::new(),
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
