use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tracing::debug;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};
use bold_proto::nfs4_proto::xdr::{nfs_resop4, nfsstat4, PUTFH4args, PUTFH4res, Serialisable};

#[async_trait]
impl NfsOperation for PUTFH4args<Bytes> {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 22: PUTFH - Set Current Filehandle {:?}, with request {:?}",
            self, request
        );

        let fh_set = request.set_filehandle(&self.object).await;

        match fh_set {
            Ok(()) => {
                // let fh_vfs = request.filehandle_vfs;
                let result: nfs_resop4<Bytes> = nfs_resop4::OP_PUTFH(PUTFH4res {
                    status: nfsstat4::NFS4_OK,
                });
                let mut buffer = BytesMut::new();
                result.serialize(&mut buffer);
                return NfsOpResponse {
                    request,
                    // result: Some(result),
                    status: nfsstat4::NFS4_OK,
                    buffer: buffer.freeze(),
                };
            }
            Err(e) => {
                let result: nfs_resop4<Bytes> =
                    nfs_resop4::OP_PUTFH(PUTFH4res { status: e.clone() });
                let mut buffer = BytesMut::new();
                result.serialize(&mut buffer);
                return NfsOpResponse {
                    request,
                    // result: Some(result),
                    status: e,
                    buffer: buffer.freeze(),
                };
            }
        }
    }
}

// #[cfg(test)]
// mod integration_tests {
//     use crate::{
//         server::{
//             nfs40::{nfs_resop4, nfsstat4, PUTFH4args, PUTFH4res},
//             operation::NfsOperation,
//         },
//         test_utils::create_nfs40_server,
//     };
//     use tracing_test::traced_test;

//     #[monoio::test]
//     #[traced_test]
//     async fn test_put_filehandle() {
//         let request = create_nfs40_server(None).await;
//         let file_manager = request.file_manager.clone();
//         let manager = file_manager.read().await;
//         let fh = manager.get_filehandle_by_path(&String::from("/"));
//         let args = PUTFH4args {
//             object: fh.unwrap().id,
//         };
//         let response = args.execute(request).await;
//         assert_eq!(response.status, nfsstat4::NFS4_OK);
//         assert_eq!(
//             response.result,
//             Some(nfs_resop4::OP_PUTFH(PUTFH4res {
//                 status: nfsstat4::NFS4_OK,
//             }))
//         );
//     }
// }
