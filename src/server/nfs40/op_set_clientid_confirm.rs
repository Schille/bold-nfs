use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use super::{NfsResOp4, NfsStat4, SetClientIdConfirm4args, SetClientIdConfirm4res};

#[async_trait]
impl NfsOperation for SetClientIdConfirm4args {
    async fn execute(&self, request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 36: SETCLIENTID_CONFIRM - Confirm Client ID {:?}, with request {:?}",
            self, request
        );

        let res = request
            .client_manager()
            .confirm_client(self.clientid, self.setclientid_confirm, None)
            .await;
        match res {
            Ok(_) => NfsOpResponse {
                request,
                result: Some(NfsResOp4::OpsetclientidConfirm(SetClientIdConfirm4res {
                    status: NfsStat4::Nfs4Ok,
                })),
                status: NfsStat4::Nfs4Ok,
            },
            Err(e) => {
                error!("Err {:?}", e);
                NfsOpResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errServerfault,
                }
            }
        }
    }
}
