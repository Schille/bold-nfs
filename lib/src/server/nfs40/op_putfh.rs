use async_trait::async_trait;
use bytes::Bytes;
use tracing::debug;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};
use bold_proto::nfs4_proto::xdr::{nfs_resop4, nfsstat4, PUTFH4args, PUTFH4res};

#[async_trait]
impl NfsOperation for PUTFH4args<Bytes> {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 22: PUTFH - Set Current Filehandle {:?}, with request {:?}",
            self, request
        );

        let fh_vfs = request.set_filehandle(&self.object).await;

        match fh_vfs {
            Ok(fh) => {
                return NfsOpResponse {
                    request,
                    result: Some(nfs_resop4::OP_PUTFH(PUTFH4res {
                        status: nfsstat4::NFS4_OK,
                    })),
                    status: nfsstat4::NFS4_OK,
                };
            }
            Err(e) => {
                return NfsOpResponse {
                    request,
                    result: Some(nfs_resop4::OP_PUTFH(PUTFH4res { status: e.clone() })),
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
            nfs40::{nfs_resop4, nfsstat4, PUTFH4args, PUTFH4res},
            operation::NfsOperation,
        },
        test_utils::create_nfs40_server,
    };
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn test_put_filehandle() {
        let request = create_nfs40_server(None).await;
        let file_manager = request.file_manager.clone();
        let manager = file_manager.read().await;
        let fh = manager.get_filehandle_by_path(&String::from("/"));
        let args = PUTFH4args {
            object: fh.unwrap().id,
        };
        let response = args.execute(request).await;
        assert_eq!(response.status, nfsstat4::NFS4_OK);
        assert_eq!(
            response.result,
            Some(nfs_resop4::OP_PUTFH(PUTFH4res {
                status: nfsstat4::NFS4_OK,
            }))
        );
    }
}
