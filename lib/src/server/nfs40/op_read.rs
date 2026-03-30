use std::io::SeekFrom;

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use futures::AsyncReadExt;
use futures::AsyncSeekExt;
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};
use bold_proto::nfs4_proto::xdr::{
    nfs_resop4, nfsstat4, READ4args, READ4res, READ4resok, Serialisable,
};

#[async_trait]
impl NfsOperation for READ4args<Bytes> {
    async fn execute<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 25: READ - Read from File {:?}, with request {:?}",
            self, request
        );
        let current_filehandle = &request.filehandle_vfs;
        let filehandle = match current_filehandle {
            Some(ref filehandle) => filehandle,
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    // result: None,
                    status: nfsstat4::NFS4ERR_FHEXPIRED,
                    buffer: Bytes::new(),
                };
            }
        };

        let count = self.count.0 as usize;
        let mut buffer = BytesMut::with_capacity(count);
        unsafe {
            buffer.set_len(count);
        }

        let mut rfile = match filehandle.file.open_file().await {
            Ok(file) => file,
            Err(e) => {
                error!("Error opening file: {:?}", e);
                return NfsOpResponse {
                    request,
                    // result: None,
                    status: nfsstat4::NFS4ERR_IO,
                    buffer: Bytes::new(),
                };
            }
        };

        if let Err(e) = rfile.seek(SeekFrom::Start(self.offset.0)).await {
            error!("Error seeking file: {:?}", e);
            return NfsOpResponse {
                request,
                // result: None,
                status: nfsstat4::NFS4ERR_IO,
                buffer: Bytes::new(),
            };
        }

        let bytes_read = match rfile.read(&mut buffer).await {
            Ok(n) => n,
            Err(e) => {
                error!("Error reading file: {:?}", e);
                return NfsOpResponse {
                    request,
                    // result: None,
                    status: nfsstat4::NFS4ERR_IO,
                    buffer: Bytes::new(),
                };
            }
        };
        buffer.truncate(bytes_read);

        let result: nfs_resop4<Bytes> = nfs_resop4::OP_READ(READ4res::NFS4_OK(READ4resok {
            eof: bytes_read < count,
            data: buffer.freeze(),
        }));
        let mut buffer = BytesMut::new();
        result.serialize(&mut buffer);

        NfsOpResponse {
            request,
            // result: Some(result),
            status: nfsstat4::NFS4_OK,
            buffer: buffer.freeze(),
        }
    }
}
