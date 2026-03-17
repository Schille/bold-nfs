use std::io::SeekFrom;

use async_trait::async_trait;
use bytes::Bytes;
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};
use bold_proto::nfs4_proto::xdr::{nfs_resop4, nfsstat4, READ4args, READ4res, READ4resok};

#[async_trait]
impl NfsOperation for READ4args<Bytes> {
    async fn execute<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 25: READ - Read from File {:?}, with request {:?}",
            self, request
        );
        let current_filehandle = request.current_filehandle_vfs();
        let filehandle = match current_filehandle {
            Some(filehandle) => filehandle,
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    result: None,
                    status: nfsstat4::NFS4ERR_FHEXPIRED,
                };
            }
        };

        let mut buffer: Vec<u8> = vec![0; self.count.0 as usize];
        let mut rfile = filehandle.file.open_file().unwrap();
        rfile.seek(SeekFrom::Start(self.offset.0)).unwrap();
        let _ = rfile.read_exact(&mut buffer);

        NfsOpResponse {
            request,
            result: Some(nfs_resop4::OP_READ(READ4res::NFS4_OK(READ4resok {
                eof: true,
                data: Bytes::from_owner(buffer),
            }))),
            status: nfsstat4::NFS4_OK,
        }
    }
}
