use async_trait::async_trait;
use tracing::debug;

use crate::server::{
    clientmanager::ClientCallback, operation::NfsOperation, request::NfsRequest,
    response::NfsOpResponse,
};

use bold_proto::nfs4_proto::{NfsResOp4, NfsStat4, SetClientId4args, SetClientId4res, SetClientId4resok};

#[async_trait]
impl NfsOperation for SetClientId4args {
    /// The client uses the SETCLIENTID operation to notify the server of its
    /// intention to use a particular client identifier, callback, and
    /// callback_ident for subsequent requests that entail creating lock,
    /// share reservation, and delegation state on the server.
    ///
    /// Please read: [RFC 7530](https://datatracker.ietf.org/doc/html/rfc7530#section-16.33)
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

#[cfg(test)]
mod integration_tests {

    use crate::{
        server::{
            nfs40::{NfsResOp4, NfsStat4, SetClientId4res},
            operation::NfsOperation,
        },
        test_utils::{create_client, create_nfs40_server},
    };

    #[tokio::test]
    async fn test_setup_new_client() {
        let request = create_nfs40_server(None).await;

        let client1 = create_client(
            [23, 213, 67, 174, 197, 95, 35, 119],
            "Linux NFSv4.0 LAPTOP/127.0.0.1".to_string(),
        );
        let client1_dup = create_client(
            [45, 5, 67, 56, 197, 6, 35, 119],
            "Linux NFSv4.0 LAPTOP/127.0.0.1".to_string(),
        );

        // run client1
        let response = client1.execute(request.clone()).await;
        let result = response.result.unwrap();
        assert_eq!(response.status, NfsStat4::Nfs4Ok);

        match result {
            NfsResOp4::Opsetclientid(res) => match res {
                SetClientId4res::Resok4(resok) => {
                    assert_eq!(resok.clientid, 1);
                    assert_eq!(resok.setclientid_confirm.len(), 8);
                }
                _ => panic!("Expected Resok4"),
            },
            _ => panic!("Expected Opsetclientid"),
        }

        // run client1_dup
        let response = client1_dup.execute(request.clone()).await;
        let result = response.result.unwrap();
        assert_eq!(response.status, NfsStat4::Nfs4Ok);

        match result {
            NfsResOp4::Opsetclientid(res) => match res {
                SetClientId4res::Resok4(resok) => {
                    // this is the same NfsClientId4.id, so it should return the same client_id
                    assert_eq!(resok.clientid, 1);
                    assert_eq!(resok.setclientid_confirm.len(), 8);
                }
                _ => panic!("Expected Resok4"),
            },
            _ => panic!("Expected Opsetclientid"),
        }

        let client2 = create_client(
            [123, 213, 2, 174, 3, 95, 5, 119],
            "Linux NFSv4.0 LAPTOP-1/127.0.0.1".to_string(),
        );

        // run client2
        let response = client2.execute(request.clone()).await;
        let result = response.result.unwrap();
        assert_eq!(response.status, NfsStat4::Nfs4Ok);

        match result {
            NfsResOp4::Opsetclientid(res) => match res {
                SetClientId4res::Resok4(resok) => {
                    assert_eq!(resok.clientid, 2);
                    assert_eq!(resok.setclientid_confirm.len(), 8);
                }
                _ => panic!("Expected Resok4"),
            },
            _ => panic!("Expected Opsetclientid"),
        }
    }
}
