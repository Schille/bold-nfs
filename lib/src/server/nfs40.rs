use async_trait::async_trait;
use bold_proto::nfs4_proto::xdr::{
    nfs_resop4, nfsstat4, utf8str_cs, COMPOUND4args, COMPOUND4res, Error, GETFH4res, GETFH4resok,
    PUTROOTFH4res, Serialisable,
};
use bold_proto::onc_rpc::auth::AuthFlavor;
use bytes::{BufMut, Bytes, BytesMut};

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
        let manager = file_manager.read().await;
        let mut buffer = BytesMut::new();
        let result: nfs_resop4<Bytes> = nfs_resop4::OP_PUTROOTFH(PUTROOTFH4res {
            status: nfsstat4::NFS4_OK,
        });
        result.serialize(&mut buffer);
        match manager.get_filehandle_by_path(&String::from("/")) {
            Some(filehandle) => {
                let _ = request.set_filehandle(&filehandle.id).await;
                NfsOpResponse {
                    request,
                    //result: Some(result),
                    status: nfsstat4::NFS4_OK,
                    buffer: buffer.freeze(),
                }
            }
            None => NfsOpResponse {
                request,
                //result: None,
                status: nfsstat4::NFS4ERR_SERVERFAULT,
                buffer: Bytes::new(),
            },
        }
    }

    fn get_current_filehandle<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        let fh = request.current_filehandle_id();
        let mut buffer = BytesMut::new();

        match fh {
            Some(filehandle_id) => {
                let result = nfs_resop4::OP_GETFH(GETFH4res::NFS4_OK(GETFH4resok {
                    object: filehandle_id,
                }));
                result.serialize(&mut buffer);
                NfsOpResponse {
                    request,
                    //result: Some(result),
                    status: nfsstat4::NFS4_OK,
                    buffer: buffer.freeze(),
                }
            }
            // current filehandle not set for client
            None => {
                error!("Filehandle not set");
                NfsOpResponse {
                    request,
                    //result: None,
                    status: nfsstat4::NFS4ERR_SERVERFAULT,
                    buffer: Bytes::new(),
                }
            }
        }
    }

    fn operation_not_supported<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        NfsOpResponse {
            request,
            //result: None,
            status: nfsstat4::NFS4ERR_NOTSUPP,
            buffer: Bytes::new(),
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
        let mut response_buffer = BytesMut::new();
        let mut last_status = nfsstat4::NFS4_OK;
        let payload = msg.payload();
        let compound_args: Result<COMPOUND4args<Bytes>, Error> =
            COMPOUND4args::try_from(payload.clone());
        let res = match compound_args {
            Ok(args) => {
                let mut resarray = Vec::with_capacity(args.argarray.len());
                // The server will process the COMPOUND procedure by evaluating each of
                // the operations within the COMPOUND procedure in order.
                debug!("Now running ops {:?}", args.argarray);
                for arg in args.argarray {
                    let response = match arg {
                        nfs_argop4::OP_GETFH => self.get_current_filehandle(request),
                        nfs_argop4::OP_SETCLIENTID(args) => args.execute(request).await,
                        nfs_argop4::OP_ACCESS(args) => args.execute(request).await,
                        nfs_argop4::OP_CLOSE(args) => args.execute(request).await,
                        nfs_argop4::OP_GETATTR(args) => args.execute(request).await,
                        nfs_argop4::OP_LOOKUP(args) => args.execute(request).await,
                        nfs_argop4::OP_OPEN(args) => args.execute(request).await,
                        nfs_argop4::OP_OPEN_CONFIRM(args) => args.execute(request).await,
                        nfs_argop4::OP_PUTFH(args) => args.execute(request).await,
                        nfs_argop4::OP_PUTROOTFH => self.put_root_filehandle(request).await,
                        nfs_argop4::OP_READ(args) => args.execute(request).await,
                        nfs_argop4::OP_READDIR(args) => args.execute(request).await,
                        nfs_argop4::OP_RENEW(args) => args.execute(request).await,
                        nfs_argop4::OP_SETCLIENTID_CONFIRM(args) => args.execute(request).await,
                        nfs_argop4::OP_SETATTR(args) => args.execute(request).await,
                        nfs_argop4::OP_REMOVE(args) => args.execute(request).await,
                        nfs_argop4::OP_WRITE(args) => args.execute(request).await,
                        nfs_argop4::OP_COMMIT(args) => args.execute(request).await,
                        nfs_argop4::OP_CREATE(args) => args.execute(request).await,
                        nfs_argop4::OP_DELEGPURGE(_) => self.operation_not_supported(request),
                        nfs_argop4::OP_DELEGRETURN(_) => self.operation_not_supported(request),
                        nfs_argop4::OP_LINK(_) => self.operation_not_supported(request),
                        nfs_argop4::OP_LOCK(_) => self.operation_not_supported(request),
                        nfs_argop4::OP_LOCKT(_) => self.operation_not_supported(request),
                        nfs_argop4::OP_LOCKU(_) => self.operation_not_supported(request),
                        nfs_argop4::OP_LOOKUPP => self.operation_not_supported(request),
                        nfs_argop4::OP_NVERIFY(_) => self.operation_not_supported(request),
                        nfs_argop4::OP_OPENATTR(_) => self.operation_not_supported(request),
                        nfs_argop4::OP_OPEN_DOWNGRADE(_) => self.operation_not_supported(request),
                        nfs_argop4::OP_PUTPUBFH => self.operation_not_supported(request),
                        nfs_argop4::OP_READLINK => self.operation_not_supported(request),
                        nfs_argop4::OP_RENAME(_) => self.operation_not_supported(request),
                        nfs_argop4::OP_RESTOREFH => self.operation_not_supported(request),
                        nfs_argop4::OP_SAVEFH => self.operation_not_supported(request),
                        nfs_argop4::OP_SECINFO(_) => self.operation_not_supported(request),
                        nfs_argop4::OP_VERIFY(_) => self.operation_not_supported(request),
                        nfs_argop4::OP_RELEASE_LOCKOWNER(_) => {
                            self.operation_not_supported(request)
                        }
                        nfs_argop4::OP_ILLEGAL => todo!(),
                    };
                    // match the result of the operation, pass on success, return on error
                    //let res = response.result;
                    last_status = response.status;
                    match last_status {
                        nfsstat4::NFS4_OK => {
                            resarray.push(1);
                            response_buffer.put(response.buffer);
                        }

                        _ => {
                            resarray.push(1);
                            let result: nfs_resop4<Bytes> = nfs_resop4::OP_ILLEGAL(ILLEGAL4res {
                                status: last_status.clone(),
                            });
                            let mut buffer = BytesMut::new();
                            result.serialize(&mut buffer);
                            response_buffer.put(buffer);
                            request = response.request;
                            break;
                        }
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
        // let nfs_response = COMPOUND4res {
        //     status: last_status,
        //     tag: utf8str_cs("".to_string()),
        //     resarray: res,
        // };
        let mut message_header = BytesMut::new();

        message_header.put_u32(last_status as u32);
        utf8str_cs("".to_string()).serialize(&mut message_header);
        // manual serialization of op responses
        message_header.put_u32(res.len() as u32);
        let p = pad_length(response_buffer.len());
        for _ in 0..p {
            response_buffer.put_u8(0)
        }
        message_header.unsplit(response_buffer);

        let rbytes = message_header.freeze();
        (
            request,
            ReplyBody::Accepted(AcceptedReply::new(
                AuthFlavor::AuthNone(None),
                AcceptedStatus::Success(rbytes),
            )),
        )
    }

    fn minor_version(&self) -> u32 {
        0
    }
}

#[inline]
fn pad_length(l: usize) -> usize {
    if l % 4 == 0 {
        return 0;
    }
    4 - (l % 4)
}
