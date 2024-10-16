use async_trait::async_trait;
use tracing::debug;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::{
    NfsResOp4, NfsStat4, OpenConfirm4args, OpenConfirm4res, OpenConfirm4resok, Stateid4,
};

#[async_trait]
impl NfsOperation for OpenConfirm4args {
    async fn execute<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 20: OPEN_CONFIRM - Confirm Open {:?}, with request {:?}",
            self, request
        );
        // we expect filehandle to have one lock (for the shared reservation)
        let lock = &request.current_filehandle().unwrap().locks[0];
        // TODO check if the stateid is correct
        NfsOpResponse {
            request,
            result: Some(NfsResOp4::OpopenConfirm(OpenConfirm4res::Resok4(
                OpenConfirm4resok {
                    open_stateid: Stateid4 {
                        seqid: lock.seqid,
                        other: lock.stateid,
                    },
                },
            ))),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
