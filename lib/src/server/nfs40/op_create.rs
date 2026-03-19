use async_trait::async_trait;
use bytes::Bytes;
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::xdr::{
    bitmap4, change_info4, changeid4, createtype4, nfs_resop4, nfsstat4, CREATE4args, CREATE4res,
    CREATE4resok,
};
use bold_proto::nfs4_utils::FileAttr;

#[async_trait]
impl NfsOperation for CREATE4args<Bytes> {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 6: CREATE - Create a Non-regular File Object {:?}, with request {:?}",
            self, request
        );

        let current_filehandle = request.current_filehandle_vfs().await;
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

        if self.objname.0 .0.len() == 0 {
            // If the objname is of zero length, NFS4ERR_INVAL will be returned.
            // The objname is also subject to the normal UTF-8, character support,
            // and name checks.  See Section 12.7 for further discussion.
            return NfsOpResponse {
                request,
                result: None,
                status: nfsstat4::NFS4ERR_INVAL,
            };
        }

        let (cinfo, attrset) = match self.objtype {
            // TODO support links
            // LinkData(vec) => todo!(),
            createtype4::NF4DIR => {
                let current_dir = if filehandle.file.is_file().await.unwrap() {
                    &filehandle.file.parent()
                } else {
                    &filehandle.file
                };
                let new_dir = current_dir.join(self.objname.0 .0.clone()).unwrap();
                let _ = new_dir.create_dir();

                // request.file_manager().touch_file(filehandle.id).await;
                let file_manager = request.file_manager.clone();
                let mut wmanager = file_manager.write().await;

                let filehandle = wmanager.get_filehandle(&new_dir).await;
                request.set_filehandle(&filehandle.id).await;

                (
                    change_info4 {
                        atomic: true,
                        before: changeid4(filehandle.attr_change(filehandle.version).await),
                        after: changeid4(filehandle.attr_change(filehandle.version).await),
                    },
                    bitmap4(Vec::new()),
                )
            }
            _ => {
                // https://datatracker.ietf.org/doc/html/rfc7530#section-16.4.2
                return NfsOpResponse {
                    request,
                    result: None,
                    status: nfsstat4::NFS4ERR_BADTYPE,
                };
            }
        };

        NfsOpResponse {
            request,
            result: Some(nfs_resop4::OP_CREATE(CREATE4res::NFS4_OK(CREATE4resok {
                cinfo,
                attrset,
            }))),
            status: nfsstat4::NFS4_OK,
        }
    }
}
