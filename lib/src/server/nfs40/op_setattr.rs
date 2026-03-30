use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tracing::{debug, error};

use crate::server::{
    nfs40::nfsstat4, operation::NfsOperation, request::NfsRequest, response::NfsOpResponse,
};

use bold_proto::nfs4_proto::xdr::{nfs_resop4, SETATTR4args, SETATTR4res, Serialisable};

#[async_trait]
impl NfsOperation for SETATTR4args<Bytes> {
    async fn execute<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 34: SETATTR - Set Attributes {:?}, with request {:?}",
            self, request
        );
        let filehandle = &request.filehandle_vfs;
        match filehandle {
            None => {
                error!("None filehandle");
                let result: nfs_resop4<Bytes> = nfs_resop4::OP_SETATTR(SETATTR4res {
                    status: nfsstat4::NFS4ERR_STALE,
                    attrsset: Vec::new().into(),
                });
                let mut buffer = BytesMut::new();
                result.serialize(&mut buffer);
                NfsOpResponse {
                    request,
                    // result: Some(result),
                    status: nfsstat4::NFS4ERR_STALE,
                    buffer: buffer.freeze(),
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

                let result: nfs_resop4<Bytes> = nfs_resop4::OP_SETATTR(SETATTR4res {
                    status: nfsstat4::NFS4_OK,
                    attrsset: attrsset.into(),
                });
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
    }
}
