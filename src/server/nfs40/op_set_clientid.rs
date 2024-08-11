use async_trait::async_trait;
use tracing::debug;

use crate::server::{
    clientmanager::ClientCallback, operation::NfsOperation, request::NfsRequest,
    response::NfsOpResponse,
};

use super::{NfsResOp4, NfsStat4, SetClientId4args, SetClientId4res, SetClientId4resok};

#[async_trait]
impl NfsOperation for SetClientId4args {
    async fn execute(&self, request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 35: SETCLIENTID - Negotiate Client ID {:?}, with request {:?}",
            self, request
        );
        let callback = ClientCallback {
            program: self.callback.cb_program,
            rnetid: self.callback.cb_location.rnetid.clone(),
            raddr: self.callback.cb_location.raddr.clone(),
            callback_ident: self.callback_ident,
        };

        let res = request
            .client_manager()
            .upsert_client(self.client.verifier, self.client.id.clone(), callback, None)
            .await;
        match res {
            Ok(client) => NfsOpResponse {
                request,
                result: Some(NfsResOp4::Opsetclientid(SetClientId4res::Resok4(
                    SetClientId4resok {
                        clientid: client.clientid,
                        setclientid_confirm: client.setclientid_confirm,
                    },
                ))),
                status: NfsStat4::Nfs4Ok,
            },
            Err(_e) => NfsOpResponse {
                request,
                result: None,
                status: NfsStat4::Nfs4errServerfault,
            },
        }
    }
}
