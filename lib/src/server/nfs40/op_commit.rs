use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::xdr::{
    nfs_resop4, nfsstat4, verifier4, COMMIT4args, COMMIT4res, COMMIT4resok, Serialisable,
};

fn verifier_from_boot(boot_time: &u64) -> [u8; 8] {
    let mut verifier = [0; 8];
    verifier.copy_from_slice(boot_time.to_be_bytes().as_ref());
    verifier
}

#[async_trait]
impl NfsOperation for COMMIT4args {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 5: COMMIT - Commit Cached Data {:?}, with request {:?}",
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

        // unlock write cache & write file

        let write_cache = request.get_current_writehandle_from_cache();
        match write_cache {
            Some(write_cache) => write_cache.commit(self.offset.0, self.count.0).await,
            None => {}
        }

        // request
        //     .file_manager()
        //     .touch_file(filehandle.id.clone())
        //     .await;

        request.drop_current_filehandle_from_cache();
        let boot_time = request.boot_time;
        let result: nfs_resop4<Bytes> = nfs_resop4::OP_COMMIT(COMMIT4res::NFS4_OK(COMMIT4resok {
            writeverf: verifier4(Bytes::copy_from_slice(&verifier_from_boot(&boot_time))),
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
