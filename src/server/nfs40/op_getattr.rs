use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    filemanager::GetFilehandleAttrsRequest, nfs40::NfsStat4, operation::NfsOperation,
    request::NfsRequest, response::NfsOpResponse,
};

use super::{Fattr4, Getattr4args, Getattr4res, Getattr4resok, NfsResOp4};

#[async_trait]
impl NfsOperation for Getattr4args {
    async fn execute(&self, request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 9: GETATTR - Get Attributes {:?}, with request {:?}",
            self, request
        );
        let filehandle = request.current_filehandle_id();
        match filehandle {
            None => {
                error!("None filehandle");
                NfsOpResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errServerfault,
                }
            }
            Some(filehandle_id) => {
                let resp = request
                    .file_manager()
                    .fmanager
                    .send(GetFilehandleAttrsRequest {
                        filehandle_id,
                        attrs_request: self.attr_request.clone(),
                    })
                    .await;
                let (answer_attrs, attrs) = match resp {
                    Ok(inner) => *inner,
                    Err(e) => {
                        error!("MailboxError {:?}", e);
                        return NfsOpResponse {
                            request,
                            result: None,
                            status: NfsStat4::Nfs4errServerfault,
                        };
                    }
                };

                NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opgetattr(Getattr4res::Resok4(Getattr4resok {
                        obj_attributes: Fattr4 {
                            attrmask: answer_attrs,
                            attr_vals: attrs,
                        },
                    }))),
                    status: NfsStat4::Nfs4Ok,
                }
            }
        }
    }
}