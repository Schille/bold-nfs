use async_trait::async_trait;

use crate::server::{
    clientmanager::{ClientCallback, UpsertClientRequest},
    operation::NfsOperation,
    request::NfsRequest,
    response::NfsResponse,
};

use super::{
    NfsResOp4, NfsStat4, SetClientId4args, SetClientId4res, SetClientId4resok,
};

#[async_trait]
impl NfsOperation for SetClientId4args {
    async fn execute(&self, request: NfsRequest) -> NfsResponse {
        let callback = ClientCallback {
            program: self.callback.cb_program,
            rnetid: self.callback.cb_location.rnetid.clone(),
            raddr: self.callback.cb_location.raddr.clone(),
            callback_ident: self.callback_ident,
        };

        let res = request
            .client_manager()
            .cmanager
            .send(UpsertClientRequest {
                verifier: self.client.verifier,
                id: self.client.id.clone(),
                callback,
                principal: None,
            })
            .await;
        match res {
            Ok(inner) => match inner {
                Ok(client) => NfsResponse {
                    request,
                    result: Some(NfsResOp4::Opsetclientid(SetClientId4res::Resok4(
                        SetClientId4resok {
                            clientid: client.clientid,
                            setclientid_confirm: client.setclientid_confirm,
                        },
                    ))),
                    status: NfsStat4::Nfs4Ok,
                },
                Err(_e) => NfsResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errServerfault,
                },
            },
            Err(_) => NfsResponse {
                request,
                result: None,
                status: NfsStat4::Nfs4errServerfault,
            },
        }
    }
}
