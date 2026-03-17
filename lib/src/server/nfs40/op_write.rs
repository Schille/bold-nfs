use std::io::{Seek, SeekFrom, Write};

use async_trait::async_trait;
use bytes::Bytes;
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::xdr::{
    count4, nfs_resop4, nfsstat4, stable_how4, verifier4, WRITE4args, WRITE4res, WRITE4resok,
};

fn verifier_from_boot(boot_time: &u64) -> Bytes {
    let mut verifier = [0; 8];
    verifier.copy_from_slice(boot_time.to_be_bytes().as_ref());
    Bytes::from_owner(verifier)
}

#[async_trait]
impl NfsOperation for WRITE4args<Bytes> {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 38: WRITE - Write to File {:?}, with request {:?}",
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

        let mut stable = stable_how4::UNSTABLE4;
        let mut count: u32 = self.data.len() as u32;
        if self.stable == stable_how4::UNSTABLE4 {
            // write to cache

            let write_cache = request.get_writehandle_from_cache(&filehandle.id);

            let write_cache = match write_cache {
                Some(write_cache) => write_cache,
                None => {
                    request.drop_filehandle_from_cache(&filehandle.id);
                    let file_manager = request.file_manager.clone();
                    let rmanager = file_manager.read().unwrap();
                    let cache_handle = rmanager.get_cache_handle(&filehandle);
                    request.cache_writehandle(filehandle.id.clone(), cache_handle.clone());
                    cache_handle
                }
            };

            write_cache.write_bytes(self.offset.0, &self.data).await;
        } else {
            // write to file
            let mut file = filehandle.file.append_file().unwrap();
            let _ = file.seek(SeekFrom::Start(self.offset.0 as u64));
            count = file.write(&self.data).unwrap() as u32;
            stable = stable_how4::FILE_SYNC4;

            if count > 0 {
                file.flush().unwrap();
                // request.file_manager().touch_file(filehandle.id).await;
            }
        }

        let boot_time = request.boot_time;
        NfsOpResponse {
            request,
            result: Some(nfs_resop4::OP_WRITE(WRITE4res::NFS4_OK(WRITE4resok {
                count: count4(count),
                committed: stable,
                writeverf: verifier4(verifier_from_boot(&boot_time)),
            }))),
            status: nfsstat4::NFS4_OK,
        }
    }
}
