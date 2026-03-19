use async_trait::async_trait;
use bytes::Bytes;
use tracing::{debug, error};

use crate::server::{
    nfs40::nfsstat4, operation::NfsOperation, request::NfsRequest, response::NfsOpResponse,
};

use bold_proto::nfs4_proto::xdr::{attrlist4, nfs_resop4, SETATTR4args, SETATTR4res};

#[async_trait]
impl NfsOperation for SETATTR4args<Bytes> {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 34: SETATTR - Set Attributes {:?}, with request {:?}",
            self, request
        );
        let filehandle = request.current_filehandle_vfs().await;
        match filehandle {
            None => {
                error!("None filehandle");
                NfsOpResponse {
                    request,
                    result: Some(nfs_resop4::OP_SETATTR(SETATTR4res {
                        status: nfsstat4::NFS4ERR_STALE,
                        attrsset: Vec::new().into(),
                    })),
                    status: nfsstat4::NFS4ERR_STALE,
                }
            }
            Some(filehandle) => {
                let attrsset = if !self.obj_attributes.attrmask.0.is_empty() {
                    let file_manager = request.file_manager.clone();
                    let rmanager = file_manager.read().await;

                    let attrsset = rmanager
                        .set_attr(
                            &filehandle,
                            &self
                                .obj_attributes
                                .attr_vals
                                .decode(&self.obj_attributes.attrmask)
                                .unwrap(),
                        )
                        .await;

                    // request
                    //     .file_manager()
                    //     .touch_file(filehandle.id.clone())
                    //     .await;

                    attrsset
                } else {
                    Vec::new()
                };

                NfsOpResponse {
                    request,
                    result: Some(nfs_resop4::OP_SETATTR(SETATTR4res {
                        status: nfsstat4::NFS4_OK,
                        attrsset: attrsset.into(),
                    })),
                    status: nfsstat4::NFS4_OK,
                }
            }
        }
    }
}
