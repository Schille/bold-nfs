use async_trait::async_trait;
use tracing::debug;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::{
    Access4args, Access4res, Access4resok, NfsResOp4, NfsStat4, ACCESS4_DELETE, ACCESS4_EXECUTE,
    ACCESS4_EXTEND, ACCESS4_LOOKUP, ACCESS4_MODIFY, ACCESS4_READ,
};

#[async_trait]
impl NfsOperation for Access4args {
    async fn execute(&self, request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 3: ACCESS - Check Access Rights {:?}, with request {:?}",
            self, request
        );
        NfsOpResponse {
            request,
            result: Some(NfsResOp4::OpAccess(Access4res::Resok4(Access4resok {
                supported: ACCESS4_READ
                    | ACCESS4_LOOKUP
                    | ACCESS4_MODIFY
                    | ACCESS4_EXTEND
                    | ACCESS4_DELETE
                    | ACCESS4_EXECUTE,
                access: self.access,
            }))),
            status: NfsStat4::Nfs4Ok,
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::{
        server::{
            nfs40::{
                Access4args, Access4res, NfsResOp4, NfsStat4, ACCESS4_DELETE, ACCESS4_EXECUTE,
                ACCESS4_EXTEND, ACCESS4_LOOKUP, ACCESS4_MODIFY, ACCESS4_READ,
            },
            operation::NfsOperation,
        },
        test_utils::create_nfs40_server,
    };
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn test_check_access() {
        let request = create_nfs40_server(None).await;
        let args = Access4args {
            access: ACCESS4_READ
                | ACCESS4_LOOKUP
                | ACCESS4_MODIFY
                | ACCESS4_EXTEND
                | ACCESS4_DELETE,
        };
        let response = args.execute(request).await;
        assert_eq!(response.status, NfsStat4::Nfs4Ok);
        if let Some(NfsResOp4::OpAccess(Access4res::Resok4(res))) = response.result {
            assert_eq!(
                res.supported,
                ACCESS4_READ
                    | ACCESS4_LOOKUP
                    | ACCESS4_MODIFY
                    | ACCESS4_EXTEND
                    | ACCESS4_DELETE
                    | ACCESS4_EXECUTE
            );
            assert_eq!(
                res.access,
                ACCESS4_READ | ACCESS4_LOOKUP | ACCESS4_MODIFY | ACCESS4_EXTEND | ACCESS4_DELETE
            );
        } else {
            panic!("Unexpected response: {:?}", response);
        }
    }
}
