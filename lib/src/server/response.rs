use bold_proto::nfs4_proto::{NfsResOp4, NfsStat4};

use super::request::NfsRequest;

#[derive(Debug)]
pub struct NfsOpResponse<'a> {
    pub request: NfsRequest<'a>,
    // result of this operation
    pub result: Option<NfsResOp4>,
    // status of this operation, err or ok
    pub status: NfsStat4,
}
