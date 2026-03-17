use async_trait::async_trait;
use bytes::Bytes;
use tracing::debug;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::xdr::{nfs_resop4, nfsstat4, stateid4, CLOSE4args, CLOSE4res};

#[async_trait]
impl NfsOperation for CLOSE4args<Bytes> {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 4: CLOSE - Close File {:?}, with request {:?}",
            self, request
        );

        let current_filehandle = request.current_filehandle_vfs().unwrap();
        request.drop_filehandle_from_cache(&current_filehandle.id);
        NfsOpResponse {
            request,
            result: Some(nfs_resop4::OP_CLOSE(CLOSE4res::NFS4_OK(stateid4 {
                seqid: self.seqid.0,
                other: self.open_stateid.clone().other,
            }))),
            status: nfsstat4::NFS4_OK,
        }
    }
}
