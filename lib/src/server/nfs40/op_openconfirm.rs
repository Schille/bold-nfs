use async_trait::async_trait;
use bytes::Bytes;
use tracing::debug;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::xdr::{
    nfs_resop4, nfsstat4, stateid4, OPEN_CONFIRM4args, OPEN_CONFIRM4res, OPEN_CONFIRM4resok,
};

#[async_trait]
impl NfsOperation for OPEN_CONFIRM4args<Bytes> {
    async fn execute<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 20: OPEN_CONFIRM - Confirm Open {:?}, with request {:?}",
            self, request
        );
        // we expect filehandle to have one lock (for the shared reservation)
        let fh = request.current_filehandle_vfs().await.unwrap();
        let file_manager = request.file_manager.clone();
        let rmanager = file_manager.read().await;
        let fh = rmanager.attach_locks(fh);
        let lock = fh.locks[0].clone();
        // TODO check if the stateid is correct
        NfsOpResponse {
            request,
            result: Some(nfs_resop4::OP_OPEN_CONFIRM(OPEN_CONFIRM4res::NFS4_OK(
                OPEN_CONFIRM4resok {
                    open_stateid: stateid4 {
                        seqid: lock.seqid,
                        other: Bytes::from_owner(lock.stateid),
                    },
                },
            ))),
            status: nfsstat4::NFS4_OK,
        }
    }
}
