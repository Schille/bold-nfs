use std::io::SeekFrom;

use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};
use bold_proto::nfs4_proto::{NfsResOp4, NfsStat4, Read4args, Read4res, Read4resok};

#[async_trait]
impl NfsOperation for Read4args {
    async fn execute(&self, request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 25: READ - Read from File {:?}, with request {:?}",
            self, request
        );
        let current_filehandle = request.current_filehandle().await;
        let filehandle = match current_filehandle {
            Some(filehandle) => filehandle,
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errFhexpired,
                };
            }
        };

        let mut buffer: Vec<u8> = vec![0; self.count as usize];
        let mut rfile = filehandle.file.open_file().unwrap();
        rfile.seek(SeekFrom::Start(self.offset)).unwrap();
        let _ = rfile.read_exact(&mut buffer);

        NfsOpResponse {
            request,
            result: Some(NfsResOp4::Opread(Read4res::Resok4(Read4resok {
                eof: true,
                data: buffer,
            }))),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
