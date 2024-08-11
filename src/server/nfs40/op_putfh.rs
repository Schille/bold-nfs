use async_trait::async_trait;
use tracing::debug;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use super::{NfsResOp4, NfsStat4, PutFh4args, PutFh4res};

#[async_trait]
impl NfsOperation for PutFh4args {
    async fn execute(&self, mut request: NfsRequest) -> NfsOpResponse {
        debug!("Operation 22: PUTFH - Set Current Filehandle {:?}, with request {:?}", self, request);
        request.set_filehandle_id(self.object.clone());
        NfsOpResponse {
            request,
            result: Some(NfsResOp4::Opputfh(PutFh4res {
                status: NfsStat4::Nfs4Ok,
            })),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
