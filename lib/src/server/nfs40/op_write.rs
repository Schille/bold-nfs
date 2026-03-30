use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use futures::AsyncWriteExt;
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::xdr::{
    count4, nfs_resop4, nfsstat4, stable_how4, verifier4, Serialisable, WRITE4args, WRITE4res,
    WRITE4resok,
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
        let mut buffer = BytesMut::new();

        let current_filehandle = &request.filehandle_vfs;
        match current_filehandle {
            Some(_) => {}
            None => {
                error!("None filehandle");
                nfsstat4::NFS4ERR_FHEXPIRED.serialize(&mut buffer);
                return NfsOpResponse {
                    request,
                    //result: None,
                    status: nfsstat4::NFS4ERR_FHEXPIRED,
                    buffer: buffer.freeze(),
                };
            }
        };

        let mut stable = stable_how4::UNSTABLE4;
        let mut count: u32 = self.data.len() as u32;
        if self.stable == stable_how4::UNSTABLE4 {
            // write to cache

            let write_cache = match request.get_current_writehandle_from_cache() {
                Some(write_cache) => write_cache,
                None => {
                    request.drop_current_filehandle_from_cache();
                    let filehandle = request.filehandle_vfs.as_ref().unwrap();
                    let file_manager = request.file_manager.clone();
                    let cache_handle = {
                        let mut wmanager = file_manager.write().await;
                        wmanager.get_cache_handle(&filehandle).await
                    };
                    request.cache_writehandle(filehandle.id.clone(), cache_handle.clone());
                    cache_handle
                }
            };

            let offset = self.offset.0;
            let data = self.data.clone();
            write_cache.write_bytes(offset, &data).await;
        } else {
            // write to file
            let filehandle = request.filehandle_vfs.as_ref().unwrap();
            let mut file = filehandle.file.append_file().await.unwrap();
            // let _ = file.seek(SeekFrom::Start(self.offset.0 as u64));
            count = file.write(&self.data).await.unwrap() as u32;
            stable = stable_how4::FILE_SYNC4;

            if count > 0 {
                file.flush().await.unwrap();
                // request.file_manager().touch_file(filehandle.id).await;
            }
        }

        let boot_time = request.boot_time;
        let result = nfs_resop4::OP_WRITE(WRITE4res::NFS4_OK(WRITE4resok {
            count: count4(count),
            committed: stable,
            writeverf: verifier4(verifier_from_boot(&boot_time)),
        }));
        let mut buffer = BytesMut::new();
        result.serialize(&mut buffer);

        NfsOpResponse {
            request,
            //result: Some(result),
            status: nfsstat4::NFS4_OK,
            buffer: buffer.freeze(),
        }
    }
}
