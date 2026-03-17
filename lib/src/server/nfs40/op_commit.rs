use async_trait::async_trait;
use bytes::Bytes;
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::xdr::{
    nfs_resop4, nfsstat4, verifier4, COMMIT4args, COMMIT4res, COMMIT4resok,
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

        // unlock write cache & write file

        let write_cache = request.get_writehandle_from_cache(&filehandle.id).unwrap();
        // // TODO: this commits the whole cache, we should only commit the data up to the offset
        write_cache.commit().await;

        // request
        //     .file_manager()
        //     .touch_file(filehandle.id.clone())
        //     .await;

        request.drop_filehandle_from_cache(&filehandle.id);
        let boot_time = request.boot_time;
        NfsOpResponse {
            request,
            result: Some(nfs_resop4::OP_COMMIT(COMMIT4res::NFS4_OK(COMMIT4resok {
                writeverf: verifier4(Bytes::copy_from_slice(&verifier_from_boot(&boot_time))),
            }))),
            status: nfsstat4::NFS4_OK,
        }
    }
}
