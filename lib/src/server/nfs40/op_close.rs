use async_trait::async_trait;
use tracing::debug;

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::{Close4args, Close4res, NfsResOp4, NfsStat4, Stateid4};

#[async_trait]
impl NfsOperation for Close4args {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 4: CLOSE - Close File {:?}, with request {:?}",
            self, request
        );

        let current_filehandle = request.current_filehandle().unwrap();
        request.drop_filehandle_from_cache(current_filehandle.id);

        NfsOpResponse {
            request,
            result: Some(NfsResOp4::Opclose(Close4res::OpenStateid(Stateid4 {
                seqid: self.seqid,
                other: self.open_stateid.other,
            }))),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
