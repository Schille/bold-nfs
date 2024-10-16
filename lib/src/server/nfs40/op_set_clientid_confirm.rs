use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::{
    NfsResOp4, NfsStat4, SetClientIdConfirm4args, SetClientIdConfirm4res,
};

#[async_trait]
impl NfsOperation for SetClientIdConfirm4args {
    async fn execute<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
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
                    status: e.nfs_error,
                }
            }
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::{
        server::{
            nfs40::{NfsResOp4, NfsStat4, SetClientId4res, SetClientIdConfirm4args},
            operation::NfsOperation,
        },
        test_utils::{create_client, create_nfs40_server},
    };
    use tracing_test::traced_test;

    fn create_client_confirm(verifier: [u8; 8], client_id: u64) -> SetClientIdConfirm4args {
        SetClientIdConfirm4args {
            clientid: client_id,
            setclientid_confirm: verifier,
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn test_confirm_clients() {
        let request = create_nfs40_server(None).await;

        let client1 = create_client(
            [23, 213, 67, 174, 197, 95, 35, 119],
            "Linux NFSv4.0 LAPTOP/127.0.0.1".to_string(),
        );
        let client2 = create_client(
            [123, 213, 2, 174, 3, 95, 5, 119],
            "Linux NFSv4.0 LAPTOP-1/127.0.0.1".to_string(),
        );

        // setup clients
        let res_client1 = client1.execute(request).await;
        let (client1_id, client1_confirm) = match res_client1.result.unwrap() {
            NfsResOp4::Opsetclientid(res) => match res {
                SetClientId4res::Resok4(resok) => (resok.clientid, resok.setclientid_confirm),
                _ => panic!("Unexpected response"),
            },
            _ => panic!("Unexpected response"),
        };

        let res_client2 = client2.execute(res_client1.request).await;
        let (client2_id, client2_confirm) = match res_client2.result.unwrap() {
            NfsResOp4::Opsetclientid(res) => match res {
                SetClientId4res::Resok4(resok) => (resok.clientid, resok.setclientid_confirm),
                _ => panic!("Unexpected response"),
            },
            _ => panic!("Unexpected response"),
        };

        // confirm client1
        let conf_client1: SetClientIdConfirm4args =
            create_client_confirm(client1_confirm, client1_id);
        let res_confirm_client1 = conf_client1.execute(res_client2.request).await;
        assert_eq!(res_confirm_client1.status, NfsStat4::Nfs4Ok);

        // confirm client2
        let conf_client2: SetClientIdConfirm4args =
            create_client_confirm(client2_confirm, client2_id);
        let res_confirm_client2 = conf_client2.execute(res_confirm_client1.request).await;
        assert_eq!(res_confirm_client2.status, NfsStat4::Nfs4Ok);

        // The server has recorded an unconfirmed { v, x, c, k, s } record and a confirmed { v, x, c, l, t } record, such that s != t
        // the server returns NFS4ERR_CLID_INUSE
        // todo: implement this test case

        // The server has no record of a confirmed or unconfirmed { *, *, c, *, s }.  The server returns NFS4ERR_STALE_CLIENTID.
        let conf_client3: SetClientIdConfirm4args =
            create_client_confirm([23, 213, 67, 174, 197, 95, 35, 119], 10);
        let res_confirm_client3 = conf_client3.execute(res_confirm_client2.request).await;
        assert_eq!(res_confirm_client3.status, NfsStat4::Nfs4errStaleClientid);
    }
}
