use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    nfs40::{ChangeInfo4, Open4res, Open4resok, OpenDelegation4, OPEN4_RESULT_CONFIRM},
    operation::NfsOperation,
    request::NfsRequest,
    response::NfsOpResponse,
};

use super::{NfsResOp4, NfsStat4, Open4args, OpenClaim4, Stateid4};

#[async_trait]
impl NfsOperation for Open4args {
    async fn execute(&self, mut request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 18: OPEN - Open a Regular File {:?}, with request {:?}",
            self, request
        );
        // open sets the current filehandle to the looked up filehandle
        let current_filehandle = request.current_filehandle().await;
        let filehandle = match current_filehandle {
            Some(filehandle) => filehandle,
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errServerfault,
                };
            }
        };

        let path = filehandle.path.clone();
        let file = &self.claim;

        match file {
            // this is open for reading
            OpenClaim4::File(file) => {
                let fh_path = {
                    if path == "/" {
                        format!("{}{}", path, file)
                    } else {
                        format!("{}/{}", path, file)
                    }
                };

                debug!("## open {:?}", fh_path);
                let filehandle = match request
                    .file_manager()
                    .get_filehandle_for_path(fh_path)
                    .await
                {
                    Ok(filehandle) => filehandle,
                    Err(e) => {
                        error!("Err {:?}", e);
                        return NfsOpResponse {
                            request,
                            result: None,
                            status: NfsStat4::Nfs4errServerfault,
                        };
                    }
                };

                request.set_filehandle_id(filehandle.id);

                NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opopen(Open4res::Resok4(Open4resok {
                        stateid: Stateid4 {
                            seqid: 0,
                            other: [0; 12],
                        },
                        cinfo: ChangeInfo4 {
                            atomic: false,
                            before: 0,
                            after: 0,
                        },
                        rflags: OPEN4_RESULT_CONFIRM,
                        attrset: Vec::new(),
                        delegation: OpenDelegation4::None,
                    }))),
                    status: NfsStat4::Nfs4Ok,
                }
            }
            // everything else is not supported
            _ => {
                todo!()
            }
        }
    }
}
