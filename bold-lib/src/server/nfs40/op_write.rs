use std::io::SeekFrom;

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
    async fn execute(&self, request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 38: WRITE - Write to File {:?}, with request {:?}",
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

        let mut file = filehandle.file.append_file().unwrap();
        let _ = file.seek(SeekFrom::Start(self.offset as u64));
        let count = file.write(&self.data).unwrap() as u32;
        let mut stable = self.stable.clone();

        //if count > 0 && (self.stable == StableHow4::DataSync4 || self.stable == StableHow4::FileSync4) {
        //    file.flush().unwrap();
        //    stable = StableHow4::FileSync4;
        //}
        if count > 0 {
            file.flush().unwrap();
            stable = StableHow4::FileSync4;
            request.file_manager().touch_file(filehandle.id).await;
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
