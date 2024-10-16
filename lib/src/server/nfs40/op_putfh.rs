use async_trait::async_trait;
use tracing::debug;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};
use bold_proto::nfs4_proto::{NfsResOp4, NfsStat4, PutFh4args, PutFh4res};

#[async_trait]
impl NfsOperation for PutFh4args {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 22: PUTFH - Set Current Filehandle {:?}, with request {:?}",
            self, request
        );

        match request.get_filehandle_from_cache(self.object.clone()) {
            Some(fh) => {
                request.set_filehandle(fh);
                return NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opputfh(PutFh4res {
                        status: NfsStat4::Nfs4Ok,
                    })),
                    status: NfsStat4::Nfs4Ok,
                };
            }
            None => {}
        }

        match request.set_filehandle_id(self.object.clone()).await {
            Ok(fh) => {
                request.cache_filehandle(fh);
                return NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opputfh(PutFh4res {
                        status: NfsStat4::Nfs4Ok,
                    })),
                    status: NfsStat4::Nfs4Ok,
                };
            }
            Err(e) => {
                return NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opputfh(PutFh4res { status: e.clone() })),
                    status: e,
                };
            }
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::{
        server::{
            nfs40::{NfsResOp4, NfsStat4, PutFh4args, PutFh4res},
            operation::NfsOperation,
        },
        test_utils::create_nfs40_server,
    };
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn test_put_filehandle() {
        let request = create_nfs40_server(None).await;
        let fh = request.file_manager().get_root_filehandle().await;

        let args = PutFh4args {
            object: fh.unwrap().id,
        };
        let response = args.execute(request).await;
        assert_eq!(response.status, NfsStat4::Nfs4Ok);
        assert_eq!(
            response.result,
            Some(NfsResOp4::Opputfh(PutFh4res {
                status: NfsStat4::Nfs4Ok,
            }))
        );
    }
}
