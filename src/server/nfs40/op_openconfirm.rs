use async_trait::async_trait;


use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsResponse};

use super::{NfsResOp4, NfsStat4, OpenConfirm4args, OpenConfirm4res, OpenConfirm4resok, Stateid4};

#[async_trait]
impl NfsOperation for OpenConfirm4args {
    async fn execute(&self, request: NfsRequest) -> NfsResponse {
        NfsResponse {
            request,
            result: Some(NfsResOp4::OpopenConfirm(OpenConfirm4res::Resok4(
                OpenConfirm4resok {
                    open_stateid: Stateid4 {
                        seqid: 0,
                        other: [0; 12],
                    },
                },
            ))),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
