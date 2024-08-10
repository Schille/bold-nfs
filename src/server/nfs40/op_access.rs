use async_trait::async_trait;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsResponse};

use super::{
    Access4args, Access4res, Access4resok, NfsResOp4, NfsStat4, ACCESS4_DELETE, ACCESS4_EXECUTE,
    ACCESS4_EXTEND, ACCESS4_LOOKUP, ACCESS4_MODIFY, ACCESS4_READ,
};

#[async_trait]
impl NfsOperation for Access4args {
    async fn execute(&self, request: NfsRequest) -> NfsResponse {
        NfsResponse {
            request,
            result: Some(NfsResOp4::OpAccess(Access4res::Resok4(Access4resok {
                supported: ACCESS4_READ
                    | ACCESS4_LOOKUP
                    | ACCESS4_MODIFY
                    | ACCESS4_EXTEND
                    | ACCESS4_DELETE
                    | ACCESS4_EXECUTE,
                access: self.access,
            }))),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
