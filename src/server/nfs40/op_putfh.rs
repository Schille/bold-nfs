use async_trait::async_trait;
use tracing::debug;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use super::{NfsResOp4, NfsStat4, PutFh4args, PutFh4res};

#[async_trait]
impl NfsOperation for PutFh4args {
    async fn execute(&self, mut request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 22: PUTFH - Set Current Filehandle {:?}, with request {:?}",
            self, request
        );
        match request
            .file_manager()
            .get_filehandle_for_id(self.object.clone())
            .await
        {
            Ok(filehandle) => {
                request.set_filehandle_id(filehandle.id.clone());
                return NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opputfh(PutFh4res {
                        status: NfsStat4::Nfs4Ok,
                    })),
                    status: NfsStat4::Nfs4Ok,
                };
            }
            Err(e) => {
                request.unset_filehandle_id();
                return NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opputfh(PutFh4res {
                        status: e.nfs_error.clone(),
                    })),
                    status: e.nfs_error,
                };
            }
        }
    }
}
