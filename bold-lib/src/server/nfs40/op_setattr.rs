use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    nfs40::NfsStat4, operation::NfsOperation, request::NfsRequest, response::NfsOpResponse,
};

use bold_proto::nfs4_proto::{NfsResOp4, SetAttr4args, SetAttr4res};

#[async_trait]
impl NfsOperation for SetAttr4args {
    async fn execute(&self, request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 34: SETATTR - Set Attributes {:?}, with request {:?}",
            self, request
        );
        let filehandle = request.current_filehandle().await;
        match filehandle {
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opsetattr(SetAttr4res {
                        status: NfsStat4::Nfs4errStale,
                        attrsset: Vec::new(),
                    })),
                    status: NfsStat4::Nfs4errStale,
                };
            }
            Some(filehandle) => {
                if !self.obj_attributes.attrmask.is_empty() {
                    // TODO implement set attr, check
                    debug!("Set attr requested for: {:?}", self.obj_attributes.attrmask);
                }

                NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opsetattr(SetAttr4res {
                        status: NfsStat4::Nfs4Ok,
                        attrsset: Vec::new(),
                    })),
                    status: NfsStat4::Nfs4Ok,
                }
            }
        }
    }
}
