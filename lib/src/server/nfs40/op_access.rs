use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tracing::debug;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::xdr::{
    nfs_resop4, nfsstat4, ACCESS4args, ACCESS4res, ACCESS4resok, Serialisable, ACCESS4_DELETE,
    ACCESS4_EXECUTE, ACCESS4_EXTEND, ACCESS4_LOOKUP, ACCESS4_MODIFY, ACCESS4_READ,
};

#[async_trait]
impl NfsOperation for ACCESS4args {
    async fn execute<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 3: ACCESS - Check Access Rights {:?}, with request {:?}",
            self, request
        );
        let result: nfs_resop4<Bytes> = nfs_resop4::OP_ACCESS(ACCESS4res::NFS4_OK(ACCESS4resok {
            supported: ACCESS4_READ
                | ACCESS4_LOOKUP
                | ACCESS4_MODIFY
                | ACCESS4_EXTEND
                | ACCESS4_DELETE
                | ACCESS4_EXECUTE,
            access: self.access,
        }));
        let mut buffer = BytesMut::new();
        result.serialize(&mut buffer);
        NfsOpResponse {
            request,
            // result: Some(result),
            status: nfsstat4::NFS4_OK,
            buffer: buffer.freeze(),
        }
    }
}

// #[cfg(test)]
// mod integration_tests {
//     use crate::{
//         server::{
//             nfs40::{
//                 nfs_resop4, nfsstat4, ACCESS4args, ACCESS4res, ACCESS4_DELETE, ACCESS4_EXECUTE,
//                 ACCESS4_EXTEND, ACCESS4_LOOKUP, ACCESS4_MODIFY, ACCESS4_READ,
//             },
//             operation::NfsOperation,
//         },
//         test_utils::create_nfs40_server,
//     };
//     use tracing_test::traced_test;

//     #[tokio::test]
//     #[traced_test]
//     async fn test_check_access() {
//         let request = create_nfs40_server(None).await;
//         let args = ACCESS4args {
//             access: ACCESS4_READ
//                 | ACCESS4_LOOKUP
//                 | ACCESS4_MODIFY
//                 | ACCESS4_EXTEND
//                 | ACCESS4_DELETE,
//         };
//         let response = args.execute(request).await;
//         assert_eq!(response.status, nfsstat4::NFS4_OK);
//         if let Some(nfs_resop4::OP_ACCESS(ACCESS4res::NFS4_OK(res))) = response.result {
//             assert_eq!(
//                 res.supported,
//                 ACCESS4_READ
//                     | ACCESS4_LOOKUP
//                     | ACCESS4_MODIFY
//                     | ACCESS4_EXTEND
//                     | ACCESS4_DELETE
//                     | ACCESS4_EXECUTE
//             );
//             assert_eq!(
//                 res.access,
//                 ACCESS4_READ | ACCESS4_LOOKUP | ACCESS4_MODIFY | ACCESS4_EXTEND | ACCESS4_DELETE
//             );
//         } else {
//             panic!("Unexpected response: {:?}", response);
//         }
//     }
// }
