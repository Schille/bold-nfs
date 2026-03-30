use bold_proto::nfs4_proto::xdr::{nfs_resop4, nfsstat4};
use bytes::Bytes;

use super::request::NfsRequest;

#[derive(Debug)]
pub struct NfsOpResponse<'a> {
    pub request: NfsRequest<'a>,
    // result of this operation
    // pub result: Option<nfs_resop4<Bytes>>,
    // status of this operation, err or ok
    pub status: nfsstat4,
    pub buffer: Bytes,
}
