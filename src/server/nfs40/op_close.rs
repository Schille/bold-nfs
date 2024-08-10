use async_trait::async_trait;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsResponse};

use super::{Close4args, Close4res, NfsResOp4, NfsStat4, Stateid4};

#[async_trait]
impl NfsOperation for Close4args {
    async fn execute(&self, request: NfsRequest) -> NfsResponse {
        NfsResponse {
            request,
            result: Some(NfsResOp4::Opclose(Close4res::OpenStateid(Stateid4 {
                seqid: 0,
                other: [0; 12],
            }))),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
