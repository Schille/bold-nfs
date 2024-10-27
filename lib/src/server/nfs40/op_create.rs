use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::{
    Attrlist4, ChangeInfo4, Create4args, Create4res, Create4resok, Createtype4, FileAttr,
    NfsResOp4, NfsStat4,
};

#[async_trait]
impl NfsOperation for Create4args {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 6: CREATE - Create a Non-regular File Object {:?}, with request {:?}",
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

        if self.objname.len() == 0 {
            // If the objname is of zero length, NFS4ERR_INVAL will be returned.
            // The objname is also subject to the normal UTF-8, character support,
            // and name checks.  See Section 12.7 for further discussion.
            return NfsOpResponse {
                request,
                result: None,
                status: NfsStat4::Nfs4errInval,
            };
        }

        let (cinfo, attrset) = match self.objtype {
            // TODO support links
            // LinkData(vec) => todo!(),
            Createtype4::Nf4dir => {
                let current_dir = if filehandle.file.is_file().unwrap() {
                    &filehandle.file.parent()
                } else {
                    &filehandle.file
                };
                let new_dir = current_dir.join(self.objname.clone()).unwrap();
                let _ = new_dir.create_dir();

                request.file_manager().touch_file(filehandle.id).await;

                let resp = request
                    .file_manager()
                    .get_filehandle_for_path(new_dir.as_str().to_string())
                    .await;
                let filehandle = match resp {
                    Ok(filehandle) => filehandle,
                    Err(e) => {
                        debug!("FileManagerError {:?}", e);
                        request.unset_filehandle();
                        return NfsOpResponse {
                            request,
                            result: None,
                            status: e.nfs_error,
                        };
                    }
                };
                request.set_filehandle(filehandle.clone());

                (
                    ChangeInfo4 {
                        atomic: true,
                        before: filehandle.attr_change,
                        after: filehandle.attr_change,
                    },
                    Attrlist4::<FileAttr>::new(None),
                )
            }
            _ => {
                // https://datatracker.ietf.org/doc/html/rfc7530#section-16.4.2
                return NfsOpResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errBadtype,
                };
            }
        };

        NfsOpResponse {
            request,
            result: Some(NfsResOp4::Opcreate(Create4res::Resok4(Create4resok {
                cinfo,
                attrset,
            }))),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
