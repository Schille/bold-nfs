use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    filemanager::GetFilehandleRequest,
    nfs40::{Lookup4res, NfsResOp4},
    operation::NfsOperation,
    request::NfsRequest,
    response::NfsResponse,
};

use super::{Lookup4args, NfsStat4};

#[async_trait]
impl NfsOperation for Lookup4args {
    async fn execute(&self, mut request: NfsRequest) -> NfsResponse {
        let current_fh = request.current_filehandle().await;
        let filehandle = match current_fh {
            Some(filehandle) => filehandle,
            None => {
                error!("None filehandle");
                return NfsResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errServerfault,
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

        let resp = request
            .file_manager()
            .fmanager
            .send(GetFilehandleRequest {
                filehandle: None,
                path: Some(path),
            })
            .await;
        let filehandle = match resp {
            Ok(filehandle) => filehandle,
            Err(e) => {
                error!("MailboxError {:?}", e);
                return NfsResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errServerfault,
                };
            }
        };

        // lookup sets the current filehandle to the looked up filehandle
        request.set_filehandle_id(filehandle.id.clone());

        NfsResponse {
            request,
            result: Some(NfsResOp4::Oplookup(Lookup4res {
                status: NfsStat4::Nfs4Ok,
            })),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
