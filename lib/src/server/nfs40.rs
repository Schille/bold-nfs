use async_trait::async_trait;
use bold_proto::nfs4_proto::xdr::{
    nfs_resop4, nfsstat4, utf8str_cs, COMPOUND4args, COMPOUND4res, Error, GETFH4res, GETFH4resok,
    PUTROOTFH4res, Serialisable,
};
use bold_proto::onc_rpc::auth::AuthFlavor;
use bytes::Bytes;

use super::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};
use bold_proto::nfs4_proto::xdr::nfs_argop4;
use bold_proto::CallBody;
use bold_proto::{nfs4_proto::xdr::*, onc_rpc::*};

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
use tracing::{debug, error};

#[derive(Debug, Clone)]
pub struct NFS40Server;

impl NFS40Server {
    async fn put_root_filehandle<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        let file_manager = request.file_manager.clone();
        let manager = file_manager.read().unwrap();

        match manager.get_filehandle_by_path(&String::from("/")) {
            Some(filehandle) => {
                request.set_filehandle(&filehandle.id);
                NfsOpResponse {
                    request,
                    result: Some(nfs_resop4::OP_PUTROOTFH(PUTROOTFH4res {
                        status: nfsstat4::NFS4_OK,
                    })),
                    status: nfsstat4::NFS4_OK,
                }
            }
            None => NfsOpResponse {
                request,
                result: None,
                status: nfsstat4::NFS4ERR_SERVERFAULT,
            },
        }
    }

    fn get_current_filehandle<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        let fh = request.current_filehandle_id();
        match fh {
            Some(filehandle_id) => NfsOpResponse {
                request,
                result: Some(nfs_resop4::OP_GETFH(GETFH4res::NFS4_OK(GETFH4resok {
                    object: filehandle_id,
                }))),
                status: nfsstat4::NFS4_OK,
            },
            // current filehandle not set for client
            None => {
                error!("Filehandle not set");
                NfsOpResponse {
                    request,
                    result: None,
                    status: nfsstat4::NFS4ERR_SERVERFAULT,
                }
            }
        }
    }

    fn operation_not_supported<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        NfsOpResponse {
            request,
            result: None,
            status: nfsstat4::NFS4ERR_NOTSUPP,
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

    async fn null<'a>(
        &self,
        _: &CallBody<Bytes, Bytes>,
        request: NfsRequest<'a>,
    ) -> (NfsRequest<'a>, ReplyBody<Bytes, Bytes>) {
        let res: COMPOUND4res<Bytes> = COMPOUND4res {
            status: nfsstat4::NFS4_OK,
            tag: utf8str_cs("".to_string()),
            resarray: Vec::new(),
        };
        let mut buf = BytesMut::new();
        res.serialize(&mut buf);
        (
            request,
            ReplyBody::Accepted(AcceptedReply::new(
                AuthFlavor::AuthNone(None),
                AcceptedStatus::Success(buf.into()),
            )),
        )
    }

    async fn compound<'a>(
        &self,
        msg: &CallBody<Bytes, Bytes>,
        mut request: NfsRequest<'a>,
    ) -> (NfsRequest<'a>, ReplyBody<Bytes, Bytes>) {
        let mut last_status = nfsstat4::NFS4_OK;
        let payload = msg.payload();
        debug!("Payload {:#x}", payload);
        let compound_args: Result<COMPOUND4args<Bytes>, Error> =
            COMPOUND4args::try_from(payload.clone());
        debug!("compound_args {:?}", compound_args);
        let res: Vec<nfs_resop4<Bytes>> = match compound_args {
            Ok(args) => {
                let mut resarray = Vec::with_capacity(args.argarray.len());
                // The server will process the COMPOUND procedure by evaluating each of
                // the operations within the COMPOUND procedure in order.
                for arg in args.argarray {
                    let response = match arg {
                        nfs_argop4::OP_GETFH => __self.get_current_filehandle(request),
                        nfs_argop4::OP_SETCLIENTID(args) => args.execute(request).await,
                        nfs_argop4::OP_ACCESS(args) => args.execute(request).await,
                        nfs_argop4::OP_CLOSE(args) => args.execute(request).await,
                        nfs_argop4::OP_GETATTR(args) => args.execute(request).await,
                        nfs_argop4::OP_LOOKUP(args) => args.execute(request).await,
                        nfs_argop4::OP_OPEN(args) => args.execute(request).await,
                        nfs_argop4::OP_OPEN_CONFIRM(args) => args.execute(request).await,
                        nfs_argop4::OP_PUTFH(args) => args.execute(request).await,
                        nfs_argop4::OP_PUTROOTFH => __self.put_root_filehandle(request).await,
                        nfs_argop4::OP_READ(args) => args.execute(request).await,
                        nfs_argop4::OP_READDIR(args) => args.execute(request).await,
                        nfs_argop4::OP_RENEW(args) => args.execute(request).await,
                        nfs_argop4::OP_SETCLIENTID_CONFIRM(args) => args.execute(request).await,
                        nfs_argop4::OP_SETATTR(args) => args.execute(request).await,
                        nfs_argop4::OP_REMOVE(args) => args.execute(request).await,
                        nfs_argop4::OP_WRITE(args) => args.execute(request).await,
                        nfs_argop4::OP_COMMIT(args) => args.execute(request).await,
                        nfs_argop4::OP_CREATE(args) => args.execute(request).await,
                        nfs_argop4::OP_DELEGPURGE(_) => __self.operation_not_supported(request),
                        nfs_argop4::OP_DELEGRETURN(_) => __self.operation_not_supported(request),
                        nfs_argop4::OP_LINK(_) => __self.operation_not_supported(request),
                        nfs_argop4::OP_LOCK(_) => __self.operation_not_supported(request),
                        nfs_argop4::OP_LOCKT(_) => __self.operation_not_supported(request),
                        nfs_argop4::OP_LOCKU(_) => __self.operation_not_supported(request),
                        nfs_argop4::OP_LOOKUPP => __self.operation_not_supported(request),
                        nfs_argop4::OP_NVERIFY(_) => __self.operation_not_supported(request),
                        nfs_argop4::OP_OPENATTR(_) => __self.operation_not_supported(request),
                        nfs_argop4::OP_OPEN_DOWNGRADE(_) => __self.operation_not_supported(request),
                        nfs_argop4::OP_PUTPUBFH => __self.operation_not_supported(request),
                        nfs_argop4::OP_READLINK => __self.operation_not_supported(request),
                        nfs_argop4::OP_RENAME(_) => __self.operation_not_supported(request),
                        nfs_argop4::OP_RESTOREFH => __self.operation_not_supported(request),
                        nfs_argop4::OP_SAVEFH => __self.operation_not_supported(request),
                        nfs_argop4::OP_SECINFO(_) => __self.operation_not_supported(request),
                        nfs_argop4::OP_VERIFY(_) => __self.operation_not_supported(request),
                        nfs_argop4::OP_RELEASE_LOCKOWNER(_) => {
                            __self.operation_not_supported(request)
                        }
                        nfs_argop4::OP_ILLEGAL => todo!(),
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
                        nfsstat4::NFS4_OK => {}
                        _ => {
                            // let mut buf = BytesMut::new();
                            // res.unwrap().serialize(&mut buf);
                            return (
                                response.request,
                                ReplyBody::Accepted(AcceptedReply::new(
                                    AuthFlavor::AuthNone(None),
                                    AcceptedStatus::SystemError,
                                )),
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
        debug!("res {:?}", res);
        let nfs_response = COMPOUND4res {
            status: last_status,
            tag: utf8str_cs("".to_string()),
            resarray: res,
        };
        let mut buf = BytesMut::new();
        nfs_response.serialize(&mut buf);
        (
            request,
            ReplyBody::Accepted(AcceptedReply::new(
                AuthFlavor::AuthNone(None),
                AcceptedStatus::Success(buf.into()),
            )),
        )
    }

    fn minor_version(&self) -> u32 {
        0
    }
}
