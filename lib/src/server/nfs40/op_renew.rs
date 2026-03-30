use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::xdr::{nfs_resop4, nfsstat4, RENEW4args, RENEW4res, Serialisable};

#[async_trait]
impl NfsOperation for RENEW4args {
    async fn execute<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 30: RENEW - Renew a Lease {:?}, with request {:?}",
            self, request
        );
        let res = request.client_manager().renew_leases(self.clientid.0).await;
        match res {
            Ok(_) => {
                let result: nfs_resop4<Bytes> = nfs_resop4::OP_RENEW(RENEW4res {
                    status: nfsstat4::NFS4_OK,
                });
                let mut buffer = BytesMut::new();
                result.serialize(&mut buffer);
                NfsOpResponse {
                    request,
                    //result: Some(result),
                    status: nfsstat4::NFS4_OK,
                    buffer: buffer.freeze(),
                }
            }
            Err(e) => {
                error!("Renew err {:?}", e);
                let result: nfs_resop4<Bytes> = nfs_resop4::OP_RENEW(RENEW4res {
                    status: e.nfs_error.clone(),
                });
                let mut buffer = BytesMut::new();
                result.serialize(&mut buffer);
                NfsOpResponse {
                    request,
                    //result: Some(result),
                    status: e.nfs_error,
                    buffer: buffer.freeze(),
                }
            }
        }
    }
}
