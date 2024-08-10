use async_trait::async_trait;
use tracing::error;

use crate::server::{
    filemanager::GetFilehandleAttrsRequest, nfs40::NfsStat4, operation::NfsOperation,
    request::NfsRequest, response::NfsResponse,
};

use super::{Fattr4, Getattr4args, Getattr4res, Getattr4resok, NfsResOp4};

#[async_trait]
impl NfsOperation for Getattr4args {
    async fn execute(&self, request: NfsRequest) -> NfsResponse {
        let filehandle = request.current_filehandle_id();
        match filehandle {
            None => {
                error!("None filehandle");
                NfsResponse {
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
                        return NfsResponse {
                            request,
                            result: None,
                            status: NfsStat4::Nfs4errServerfault,
                        };
                    }
                };

                NfsResponse {
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
