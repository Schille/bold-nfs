use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::{NfsResOp4, NfsStat4, Renew4args, Renew4res};

#[async_trait]
impl NfsOperation for Renew4args {
    async fn execute(&self, request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 30: RENEW - Renew a Lease {:?}, with request {:?}",
            self, request
        );
        let res = request.client_manager().renew_leases(self.clientid).await;
        match res {
            Ok(_) => NfsOpResponse {
                request,
                result: Some(NfsResOp4::Oprenew(Renew4res {
                    status: NfsStat4::Nfs4Ok,
                })),
                status: NfsStat4::Nfs4Ok,
            },
            Err(e) => {
                error!("Renew err {:?}", e);
                NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Oprenew(Renew4res {
                        status: e.nfs_error.clone(),
                    })),
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
            nfs40::{NfsResOp4, NfsStat4, Renew4args, SetClientId4res, SetClientIdConfirm4args},
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

        // setup client
        let res_client1 = client1.execute(request.clone()).await;
        let (client1_id, client1_confirm) = match res_client1.result.unwrap() {
            NfsResOp4::Opsetclientid(res) => match res {
                SetClientId4res::Resok4(resok) => (resok.clientid, resok.setclientid_confirm),
                _ => panic!("Unexpected response"),
            },
            _ => panic!("Unexpected response"),
        };

        // confirm client1
        let conf_client1: SetClientIdConfirm4args =
            create_client_confirm(client1_confirm, client1_id);
        let res_confirm_client1 = conf_client1.execute(request.clone()).await;
        assert_eq!(res_confirm_client1.status, NfsStat4::Nfs4Ok);

        // renew client1
        let renew_client1 = Renew4args {
            clientid: client1_id,
        };
        let res_renew_client1 = renew_client1.execute(request.clone()).await;
        assert_eq!(res_renew_client1.status, NfsStat4::Nfs4Ok);

        // renew stale client
        let renew_client2 = Renew4args { clientid: 50 };
        let res_renew_client2 = renew_client2.execute(request.clone()).await;
        assert_eq!(res_renew_client2.status, NfsStat4::Nfs4errStaleClientid);
    }
}
