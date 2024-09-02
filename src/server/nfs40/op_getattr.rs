use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    nfs40::NfsStat4, operation::NfsOperation, request::NfsRequest, response::NfsOpResponse,
};

use super::{Fattr4, Getattr4args, Getattr4resok, NfsResOp4};

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
                return NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opgetattr(Getattr4resok {
                        obj_attributes: None,
                        status: NfsStat4::Nfs4errStale,
                    })),
                    status: NfsStat4::Nfs4errStale,
                };
            }
            Some(filehandle_id) => {
                let resp = request
                    .file_manager()
                    .get_filehandle_attrs(filehandle_id, self.attr_request.clone())
                    .await;
                let (answer_attrs, attrs) = match resp {
                    Ok(inner) => *inner,
                    Err(e) => {
                        error!("FileManagerError {:?}", e);
                        return NfsOpResponse {
                            request,
                            result: Some(NfsResOp4::Opgetattr(Getattr4resok {
                                obj_attributes: None,
                                status: e.nfs_error.clone(),
                            })),
                            status: e.nfs_error,
                        };
                    }
                };

                NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opgetattr(Getattr4resok {
                        status: NfsStat4::Nfs4Ok,
                        obj_attributes: Some(Fattr4 {
                            attrmask: answer_attrs,
                            attr_vals: attrs,
                        }),
                    })),
                    status: NfsStat4::Nfs4Ok,
                }
            }
        }
    }
}
