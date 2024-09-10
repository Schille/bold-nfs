use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    nfs40::{Lookup4res, NfsResOp4},
    operation::NfsOperation,
    request::NfsRequest,
    response::NfsOpResponse,
};

use super::{Lookup4args, NfsStat4};

#[async_trait]
impl NfsOperation for Lookup4args {
    async fn execute(&self, mut request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 15: LOOKUP - Look Up Filename {:?}, with request {:?}",
            self, request
        );
        let current_fh = request.current_filehandle().await;
        let filehandle = match current_fh {
            Some(filehandle) => filehandle,
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errFhexpired,
                };
            }
        };

        let mut path = filehandle.path.clone();
        if path == "/" {
            path.push_str(self.objname.as_str());
        } else {
            path.push('/');
            path.push_str(self.objname.as_str());
        }

        debug!("lookup {:?}", path);

        let resp = request.file_manager().get_filehandle_for_path(path).await;
        let filehandle = match resp {
            Ok(filehandle) => filehandle,
            Err(e) => {
                error!("FileManagerError {:?}", e);
                request.unset_filehandle_id();
                return NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Oplookup(Lookup4res {
                        status: e.nfs_error.clone(),
                    })),
                    status: e.nfs_error,
                };
            }
        };

        // lookup sets the current filehandle to the looked up filehandle
        request.set_filehandle_id(filehandle.id);

        NfsOpResponse {
            request,
            result: Some(NfsResOp4::Oplookup(Lookup4res {
                status: NfsStat4::Nfs4Ok,
            })),
            status: NfsStat4::Nfs4Ok,
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::{
        server::{
            nfs40::{Lookup4args, NfsStat4, PutFh4args},
            operation::NfsOperation,
        },
        test_utils::{create_fake_fs, create_nfs40_server},
    };
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn test_lookup() {
        let request = create_nfs40_server(Some(create_fake_fs())).await;
        let fh = request.file_manager().get_root_filehandle().await;

        let putfh_args = PutFh4args {
            object: fh.clone().unwrap().id,
        };
        let putfh_request = putfh_args.execute(request).await;

        let args = Lookup4args {
            objname: "file1.txt".to_string(),
        };
        let lookup1_response = args.execute(putfh_request.request).await;
        assert_eq!(lookup1_response.status, NfsStat4::Nfs4Ok);
        // if let Some(NfsResOp4::OpAccess(Access4res::Resok4(res))) = response.result {
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

        let putfh_args = PutFh4args {
            object: fh.unwrap().id,
        };
        let putfh1_request = putfh_args.execute(lookup1_response.request).await;

        let args = Lookup4args {
            objname: "doesnotexist".to_string(),
        };
        let lookup2_response = args.execute(putfh1_request.request).await;
        assert_eq!(lookup2_response.status, NfsStat4::Nfs4errStale);
    }
}
