use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tracing::debug;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::xdr::{
    nfs_resop4, nfsstat4, stateid4, CLOSE4args, CLOSE4res, Serialisable,
};

#[async_trait]
impl NfsOperation for CLOSE4args<Bytes> {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 4: CLOSE - Close File {:?}, with request {:?}",
            self, request
        );

        request.drop_current_filehandle_from_cache();
        let result: nfs_resop4<Bytes> = nfs_resop4::OP_CLOSE(CLOSE4res::NFS4_OK(stateid4 {
            seqid: self.seqid.0,
            other: self.open_stateid.clone().other,
        }));
        let mut buffer = BytesMut::new();
        result.serialize(&mut buffer);

        NfsOpResponse {
            request,
            // result: Some(result),
            status: nfsstat4::NFS4_OK,
            buffer: buffer.freeze(),
        }
    }
}
