use async_trait::async_trait;
use tracing::error;

use crate::server::{
    clientmanager::ConfirmClientRequest, operation::NfsOperation, request::NfsRequest,
    response::NfsResponse,
};

use super::{NfsResOp4, NfsStat4, SetClientIdConfirm4args, SetClientIdConfirm4res};

#[async_trait]
impl NfsOperation for SetClientIdConfirm4args {
    async fn execute(&self, request: NfsRequest) -> NfsResponse {
        let client_id = self.clientid;
        let setclientid_confirm = self.setclientid_confirm;

        let res = request
            .client_manager()
            .cmanager
            .send(ConfirmClientRequest {
                client_id,
                setclientid_confirm,
                principal: None,
            })
            .await;
        match res {
            Ok(inner) => match inner {
                Ok(_) => NfsResponse {
                    request,
                    result: Some(NfsResOp4::OpsetclientidConfirm(SetClientIdConfirm4res {
                        status: NfsStat4::Nfs4Ok,
                    })),
                    status: NfsStat4::Nfs4Ok,
                },
                Err(e) => {
                    error!("Err {:?}", e);
                    NfsResponse {
                        request,
                        result: None,
                        status: NfsStat4::Nfs4errServerfault,
                    }
                }
            },
            Err(e) => {
                error!("MailboxError {:?}", e);
                NfsResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errServerfault,
                }
            }
        }
    }
}
