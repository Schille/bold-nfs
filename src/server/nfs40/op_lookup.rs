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
