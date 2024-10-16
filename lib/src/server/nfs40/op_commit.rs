
use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::{Commit4args, Commit4res, Commit4resok, NfsResOp4, NfsStat4};

fn verifier_from_boot(boot_time: &u64) -> [u8; 8] {
    let mut verifier = [0; 8];
    verifier.copy_from_slice(boot_time.to_be_bytes().as_ref());
    verifier
}

#[async_trait]
impl NfsOperation for Commit4args {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 5: COMMIT - Commit Cached Data {:?}, with request {:?}",
            self, request
        );
        let current_filehandle = request.current_filehandle();
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

        // unlock write cache & write file
        request.write_cache_commit(&filehandle.file);

        request
            .file_manager()
            .touch_file(filehandle.id.clone())
            .await;

        // let write_cache = request
        //     .file_manager()
        //     .get_write_cache_handle(filehandle.clone())
        //     .await
        //     .unwrap();
        // // TODO: this commits the whole cache, we should only commit the data up to the offset
        // write_cache.commit().await;
        request.drop_filehandle_from_cache(filehandle.id);

        let boot_time = request.boot_time;
        NfsOpResponse {
            request,
            result: Some(NfsResOp4::Opcommit(Commit4res::Resok4(Commit4resok {
                writeverf: verifier_from_boot(&boot_time),
            }))),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
