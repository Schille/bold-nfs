use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    nfs40::{nfs_resop4, LOOKUP4res},
    operation::NfsOperation,
    request::NfsRequest,
    response::NfsOpResponse,
};

use bold_proto::nfs4_proto::xdr::{nfsstat4, LOOKUP4args};

#[async_trait]
impl NfsOperation for LOOKUP4args {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 15: LOOKUP - Look Up Filename {:?}, with request {:?}",
            self, request
        );
        let current_fh = request.current_filehandle_vfs();
        let filehandle = match current_fh {
            Some(filehandle) => filehandle,
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    result: None,
                    status: nfsstat4::NFS4ERR_FHEXPIRED,
                };
            }
        };

        let mut path = filehandle.path.clone();
        if path == "/" {
            path.push_str(self.objname.0 .0.as_str());
        } else {
            path.push('/');
            path.push_str(self.objname.0 .0.as_str());
        }

        debug!("lookup {:?}", path);

        let file_manager = request.file_manager.clone();
        let rmanager = file_manager.read().unwrap();
        let fh = rmanager.get_filehandle_by_path(&path);
        drop(rmanager);

        let filehandle = match fh {
            Some(filehandle) => Some(filehandle),
            None => {
                // a missing file during lookup is not an error

                let lookup_miss = filehandle.file.join(path).unwrap();
                if lookup_miss.exists().unwrap() {
                    let mut wmanager = file_manager.write().unwrap();
                    let fh = wmanager.get_filehandle(&lookup_miss);
                    drop(wmanager);
                    Some(fh)
                } else {
                    None
                }
            }
        };

        if filehandle.is_none() {
            request.unset_filehandle();
            return NfsOpResponse {
                request,
                result: Some(nfs_resop4::OP_LOOKUP(LOOKUP4res {
                    status: nfsstat4::NFS4ERR_NOENT,
                })),
                status: nfsstat4::NFS4ERR_NOENT,
            };
        }

        // lookup sets the current filehandle to the looked up filehandle
        let _ = request.set_filehandle(&filehandle.unwrap().id);

        NfsOpResponse {
            request,
            result: Some(nfs_resop4::OP_LOOKUP(LOOKUP4res {
                status: nfsstat4::NFS4_OK,
            })),
            status: nfsstat4::NFS4_OK,
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::{
        server::{
            nfs40::{nfsstat4, LOOKUP4args, PUTFH4args},
            operation::NfsOperation,
        },
        test_utils::{create_fake_fs, create_nfs40_server},
    };
    use bold_proto::nfs4_proto::xdr::{component4, utf8str_cs};
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn test_lookup() {
        let request = create_nfs40_server(Some(create_fake_fs())).await;
        let file_manager = request.file_manager.clone();
        let manager = file_manager.read().unwrap();
        let fh = manager.get_filehandle_by_path(&String::from("/"));

        let putfh_args = PUTFH4args {
            object: fh.clone().unwrap().id,
        };
        drop(manager);
        let putfh_request = putfh_args.execute(request).await;

        let args = LOOKUP4args {
            objname: component4(utf8str_cs("file1.txt".to_string())),
        };
        let lookup1_response = args.execute(putfh_request.request).await;
        assert_eq!(lookup1_response.status, nfsstat4::NFS4_OK);
        // if let Some(nfs_resop4::OpAccess(Access4res::Resok4(res))) = response.result {
        //     assert_eq!(
        //         res.supported,
        //         ACCESS4_READ
        //             | ACCESS4_LOOKUP
        //             | ACCESS4_MODIFY
        //             | ACCESS4_EXTEND
        //             | ACCESS4_DELETE
        //             | ACCESS4_EXECUTE
        //     );
        //     assert_eq!(
        //         res.access,
        //         ACCESS4_READ | ACCESS4_LOOKUP | ACCESS4_MODIFY | ACCESS4_EXTEND | ACCESS4_DELETE
        //     );
        // } else {
        //     panic!("Unexpected response: {:?}", response);
        // }

        let putfh_args = PUTFH4args {
            object: fh.unwrap().id,
        };
        let putfh1_request = putfh_args.execute(lookup1_response.request).await;

        let args = LOOKUP4args {
            objname: component4(utf8str_cs("doesnotexist".to_string())),
        };
        let lookup2_response = args.execute(putfh1_request.request).await;
        assert_eq!(lookup2_response.status, nfsstat4::NFS4ERR_NOENT);
    }
}
