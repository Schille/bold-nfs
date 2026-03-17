use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    nfs40::{change_info4, nfsstat4},
    operation::NfsOperation,
    request::NfsRequest,
    response::NfsOpResponse,
};

use bold_proto::nfs4_proto::xdr::{changeid4, nfs_resop4, REMOVE4args, REMOVE4res, REMOVE4resok};

#[async_trait]
impl NfsOperation for REMOVE4args {
    async fn execute<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 28: REMOVE - Remove File System Object {:?}, with request {:?}",
            self, request
        );
        let filehandle = request.current_filehandle_vfs();
        match filehandle {
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    result: Some(nfs_resop4::OP_REMOVE(REMOVE4res::NFS4_OK(REMOVE4resok {
                        cinfo: change_info4 {
                            atomic: false,
                            before: changeid4(0),
                            after: changeid4(0),
                        },
                    }))),
                    status: nfsstat4::NFS4ERR_STALE,
                };
            }
            Some(filehandle) => {
                let target = filehandle.file.join(self.target.0 .0.clone()).unwrap();
                let file_manager = request.file_manager.clone();
                let mut wmanager = file_manager.write().unwrap();
                let target_filehandle = wmanager.get_filehandle(&target);
                wmanager.remove_file(target_filehandle);
                NfsOpResponse {
                    request,
                    result: Some(nfs_resop4::OP_REMOVE(REMOVE4res::NFS4_OK(REMOVE4resok {
                        cinfo: change_info4 {
                            atomic: false,
                            before: changeid4(0),
                            after: changeid4(0),
                        },
                    }))),
                    status: nfsstat4::NFS4ERR_STALE,
                }
            }
        }
    }
}
