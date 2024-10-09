use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    nfs40::{ChangeInfo4, NfsStat4},
    operation::NfsOperation,
    request::NfsRequest,
    response::NfsOpResponse,
};

use bold_proto::nfs4_proto::{NfsResOp4, Remove4args, Remove4res};

#[async_trait]
impl NfsOperation for Remove4args {
    async fn execute(&self, request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 28: REMOVE - Remove File System Object {:?}, with request {:?}",
            self, request
        );
        let filehandle = request.current_filehandle().await;
        match filehandle {
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opremove(Remove4res {
                        status: NfsStat4::Nfs4errStale,
                        cinfo: ChangeInfo4 {
                            atomic: false,
                            before: 0,
                            after: 0,
                        },
                    })),
                    status: NfsStat4::Nfs4errStale,
                };
            }
            Some(filehandle) => {
                let path = filehandle.file.join(self.target.clone()).unwrap();
                let res = request.file_manager().remove_file(path).await;
                match res {
                    Ok(_) => NfsOpResponse {
                        request,
                        result: Some(NfsResOp4::Opremove(Remove4res {
                            status: NfsStat4::Nfs4Ok,
                            cinfo: ChangeInfo4 {
                                atomic: false,
                                before: 0,
                                after: 0,
                            },
                        })),
                        status: NfsStat4::Nfs4errStale,
                    },
                    Err(_) => todo!(),
                }
            }
        }
    }
}
