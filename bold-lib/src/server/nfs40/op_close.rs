use async_trait::async_trait;
use tracing::debug;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use super::{Close4args, Close4res, NfsResOp4, NfsStat4, Stateid4};

#[async_trait]
impl NfsOperation for Close4args {
    async fn execute(&self, request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 4: CLOSE - Close File {:?}, with request {:?}",
            self, request
        );
        NfsOpResponse {
            request,
            result: Some(NfsResOp4::Opclose(Close4res::OpenStateid(Stateid4 {
                seqid: self.seqid,
                other: self.open_stateid.other,
            }))),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
