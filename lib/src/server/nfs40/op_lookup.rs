use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tracing::{debug, error};

use crate::server::{
    nfs40::{nfs_resop4, LOOKUP4res},
    operation::NfsOperation,
    request::NfsRequest,
    response::NfsOpResponse,
};

use bold_proto::nfs4_proto::xdr::{nfsstat4, LOOKUP4args, Serialisable};

#[async_trait]
impl NfsOperation for LOOKUP4args {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 15: LOOKUP - Look Up Filename {:?}, with request {:?}",
            self, request
        );
        let current_fh = &request.filehandle_vfs;
        let filehandle = match current_fh {
            Some(ref filehandle) => filehandle,
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    // result: None,
                    status: nfsstat4::NFS4ERR_FHEXPIRED,
                    buffer: Bytes::new(),
                };
            }
        };

        let objname = self.objname.0 .0.as_str();
        let path = if filehandle.path == "/" {
            format!("/{}", objname)
        } else {
            format!("{}/{}", filehandle.path, objname)
        };

        debug!("lookup {:?}", path);

        let file_manager = request.file_manager.clone();
        let rmanager = file_manager.read().await;
        let fh = rmanager.get_filehandle_by_path(&path);
        drop(rmanager);

        debug!("lookup fh {:?}", fh);

        let filehandle = match fh {
            Some(filehandle) => Some(filehandle),
            None => {
                // a missing file during lookup is not an error

                let lookup_miss = filehandle.file.join(&path).unwrap();
                if lookup_miss.exists().await.unwrap() {
                    let mut wmanager = file_manager.write().await;
                    let fh = wmanager.get_filehandle(&lookup_miss).await;
                    drop(wmanager);
                    Some(fh)
                } else {
                    None
                }
            }
        };

        if filehandle.is_none() {
            request.unset_filehandle();
            debug!("lookup unset");
            let result: nfs_resop4<Bytes> = nfs_resop4::OP_LOOKUP(LOOKUP4res {
                status: nfsstat4::NFS4ERR_NOENT,
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

        // lookup sets the current filehandle to the looked up filehandle
        let _ = request.set_filehandle(&filehandle.unwrap().id).await;
        debug!("lookup done");
        let result: nfs_resop4<Bytes> = nfs_resop4::OP_LOOKUP(LOOKUP4res {
            status: nfsstat4::NFS4_OK,
        });
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
//             nfs40::{nfs_resop4, nfsstat4, LOOKUP4args, LOOKUP4res, PUTFH4args},
//             operation::NfsOperation,
//         },
//         test_utils::{create_fake_fs, create_nfs40_server},
//     };
//     use bold_proto::nfs4_proto::xdr::{component4, utf8str_cs};
//     use tracing_test::traced_test;

//     #[tokio::test]
//     #[traced_test]
//     async fn test_lookup() {
//         let request = create_nfs40_server(Some(create_fake_fs().await)).await;
//         let file_manager = request.file_manager.clone();
//         let manager = file_manager.read().await;
//         let fh = manager.get_filehandle_by_path(&String::from("/"));

//         let putfh_args = PUTFH4args {
//             object: fh.as_ref().unwrap().id.clone(),
//         };
//         drop(manager);
//         let putfh_request = putfh_args.execute(request).await;

//         let args = LOOKUP4args {
//             objname: component4(utf8str_cs("file1.txt".to_string())),
//         };
//         let lookup1_response = args.execute(putfh_request.request).await;
//         assert_eq!(lookup1_response.status, nfsstat4::NFS4_OK);
//         // if let Some(nfs_resop4::OpAccess(Access4res::Resok4(res))) = response.result {
//         //     assert_eq!(
//         //         res.supported,
//         //         ACCESS4_READ
//         //             | ACCESS4_LOOKUP
//         //             | ACCESS4_MODIFY
//         //             | ACCESS4_EXTEND
//         //             | ACCESS4_DELETE
//         //             | ACCESS4_EXECUTE
//         //     );
//         //     assert_eq!(
//         //         res.access,
//         //         ACCESS4_READ | ACCESS4_LOOKUP | ACCESS4_MODIFY | ACCESS4_EXTEND | ACCESS4_DELETE
//         //     );
//         // } else {
//         //     panic!("Unexpected response: {:?}", response);
//         // }

//         let putfh_args = PUTFH4args {
//             object: fh.unwrap().id,
//         };
//         let putfh1_request = putfh_args.execute(lookup1_response.request).await;

//         let args = LOOKUP4args {
//             objname: component4(utf8str_cs("doesnotexist".to_string())),
//         };
//         let lookup2_response = args.execute(putfh1_request.request).await;
//         assert_eq!(lookup2_response.status, nfsstat4::NFS4_OK);
//         if let Some(nfs_resop4::OP_LOOKUP(res)) = lookup2_response.result {
//             assert_eq!(res.status, nfsstat4::NFS4ERR_NOENT);
//         } else {
//             panic!("Unexpected result");
//         }
//     }
// }
