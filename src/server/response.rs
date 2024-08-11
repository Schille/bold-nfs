use crate::proto::nfs4_proto::{NfsResOp4, NfsStat4};

use super::request::NfsRequest;

#[derive(Debug)]
pub struct NfsOpResponse {
    pub request: NfsRequest,
    // result of this operation
    pub result: Option<NfsResOp4>,
    // status of this operation, err or ok
    pub status: NfsStat4,
}
