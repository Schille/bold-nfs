use std::io::{Seek, SeekFrom, Write};

use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::{NfsResOp4, NfsStat4, StableHow4, Write4args, Write4res, Write4resok};

fn verifier_from_boot(boot_time: &u64) -> [u8; 8] {
    let mut verifier = [0; 8];
    verifier.copy_from_slice(boot_time.to_be_bytes().as_ref());
    verifier
}

#[async_trait]
impl NfsOperation for Write4args {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 38: WRITE - Write to File {:?}, with request {:?}",
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

        let mut stable = StableHow4::Unstable4;
        let mut count: u32 = self.data.len() as u32;
        if self.stable == StableHow4::Unstable4 {
            // write to cache
            let write_cache = match &filehandle.write_cache {
                Some(write_cache) => write_cache,
                None => {
                    let write_cache = request
                        .file_manager()
                        .get_write_cache_handle(filehandle.clone())
                        .await
                        .unwrap();
                    request.drop_filehandle_from_cache(filehandle.id.clone());
                    &write_cache.clone()
                }
            };

            write_cache
                .write_bytes(self.offset, self.data.clone())
                .await;
        } else {
            // write to file
            let mut file = filehandle.file.append_file().unwrap();
            let _ = file.seek(SeekFrom::Start(self.offset as u64));
            count = file.write(&self.data).unwrap() as u32;
            stable = StableHow4::FileSync4;

            if count > 0 {
                file.flush().unwrap();
                request.file_manager().touch_file(filehandle.id).await;
            }
        }

        let boot_time = request.boot_time;
        NfsOpResponse {
            request,
            result: Some(NfsResOp4::Opwrite(Write4res::Resok4(Write4resok {
                count: count,
                committed: stable,
                writeverf: verifier_from_boot(&boot_time),
            }))),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
