use async_trait::async_trait;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsResponse};

use super::{NfsResOp4, NfsStat4, PutFh4args, PutFh4res};

#[async_trait]
impl NfsOperation for PutFh4args {
    async fn execute(&self, request: NfsRequest) -> NfsResponse {
        NfsResponse {
            request,
            result: Some(NfsResOp4::Opputfh(PutFh4res {
                status: NfsStat4::Nfs4Ok,
            })),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
