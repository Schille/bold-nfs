use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    filemanager::Filehandle,
    nfs40::{ChangeInfo4, Open4res, Open4resok, OpenDelegation4, OPEN4_RESULT_CONFIRM},
    operation::NfsOperation,
    request::NfsRequest,
    response::NfsOpResponse,
};

use super::{CreateHow4, NfsResOp4, NfsStat4, Open4args, OpenClaim4, OpenFlag4, Stateid4};

impl Open4args {
    async fn open_for_reading(
        &self,
        filehandle: Box<Filehandle>,
        file: &String,
        mut request: NfsRequest,
    ) -> NfsOpResponse {
        let path = filehandle.path;

        let fh_path = {
            if path == "/" {
                format!("{}{}", path, file)
            } else {
                format!("{}/{}", path, file)
            }
        };

        debug!("open_for_reading {:?}", fh_path);
        let filehandle = match request
            .file_manager()
            .get_filehandle_for_path(fh_path)
            .await
        {
            Ok(filehandle) => filehandle,
            Err(e) => {
                error!("Err {:?}", e);
                return NfsOpResponse {
                    request,
                    result: None,
                    status: e.nfs_error,
                };
            }
        };

        request.set_filehandle_id(filehandle.id);

        NfsOpResponse {
            request,
            result: Some(NfsResOp4::Opopen(Open4res::Resok4(Open4resok {
                stateid: Stateid4 {
                    seqid: 0,
                    other: [0; 12],
                },
                cinfo: ChangeInfo4 {
                    atomic: false,
                    before: 0,
                    after: 0,
                },
                // OPEN4_RESULT_CONFIRM indicates that the client MUST execute an
                // OPEN_CONFIRM operation before using the open file.
                rflags: OPEN4_RESULT_CONFIRM,
                attrset: Vec::new(),
                delegation: OpenDelegation4::None,
            }))),
            status: NfsStat4::Nfs4Ok,
        }
    }

    async fn open_for_writing(
        &self,
        filehandle: Box<Filehandle>,
        file: &String,
        how: &CreateHow4,
        mut request: NfsRequest,
    ) -> NfsOpResponse {
        let path = filehandle.path;

        let fh_path = {
            if path == "/" {
                format!("{}{}", path, file)
            } else {
                format!("{}/{}", path, file)
            }
        };

        debug!("open_for_writing {:?}", fh_path);

        let newfile_op = filehandle.file.join(file);

        let filehandle = match how {
            CreateHow4::UNCHECKED4(_fattr) => {
                match request
                    .file_manager()
                    .create_file(
                        newfile_op.unwrap(),
                        self.owner.clientid,
                        self.owner.owner.clone(),
                        self.share_access,
                        self.share_deny,
                        None,
                    )
                    .await
                {
                    Ok(filehandle) => filehandle,
                    Err(e) => {
                        error!("Err {:?}", e);
                        return NfsOpResponse {
                            request,
                            result: None,
                            status: NfsStat4::Nfs4errServerfault,
                        };
                    }
                }
            }
            CreateHow4::EXCLUSIVE4(verifier) => {
                match request
                    .file_manager()
                    .create_file(
                        newfile_op.unwrap(),
                        self.owner.clientid,
                        self.owner.owner.clone(),
                        self.share_access,
                        self.share_deny,
                        Some(*verifier),
                    )
                    .await
                {
                    Ok(filehandle) => filehandle,
                    Err(e) => {
                        error!("Err {:?}", e);
                        return NfsOpResponse {
                            request,
                            result: None,
                            status: NfsStat4::Nfs4errServerfault,
                        };
                    }
                }
            }
            _ => {
                error!("Unsupported CreateHow4 {:?}", how);
                return NfsOpResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errNotsupp,
                };
            }
        };

        request.set_filehandle_id(filehandle.id);
        // we expect this filehandle to have one lock (for the shared reservation)
        let lock = &filehandle.locks[0];

        NfsOpResponse {
            request,
            result: Some(NfsResOp4::Opopen(Open4res::Resok4(Open4resok {
                stateid: Stateid4 {
                    seqid: lock.seqid,
                    other: lock.stateid,
                },
                cinfo: ChangeInfo4 {
                    atomic: false,
                    before: 0,
                    after: 0,
                },
                // OPEN4_RESULT_CONFIRM indicates that the client MUST execute an
                // OPEN_CONFIRM operation before using the open file.
                rflags: OPEN4_RESULT_CONFIRM,
                attrset: Vec::new(),
                delegation: OpenDelegation4::None,
            }))),
            status: NfsStat4::Nfs4Ok,
        }
    }
}

#[async_trait]
impl NfsOperation for Open4args {
    async fn execute(&self, request: NfsRequest) -> NfsOpResponse {
        // Description: https://datatracker.ietf.org/doc/html/rfc7530#section-16.16.5
        debug!(
            "Operation 18: OPEN - Open a Regular File {:?}, with request {:?}",
            self, request
        );
        // open sets the current filehandle to the looked up filehandle
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

        // If the current filehandle is not a directory, the error
        // NFS4ERR_NOTDIR will be returned.
        if !filehandle.file.is_dir().unwrap() {
            error!("Not a directory");
            return NfsOpResponse {
                request,
                result: None,
                status: NfsStat4::Nfs4errNotdir,
            };
        }

        let file = match &self.claim {
            // CLAIM_NULL:  For the client, this is a new OPEN request, and there is
            // no previous state associated with the file for the client.
            OpenClaim4::ClaimNull(file) => file,
            // NFS4ERR_NOTSUPP is returned if the server does not support this
            // claim type.
            _ => {
                error!("Unsupported OpenClaim4 {:?}", self.claim);
                return NfsOpResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errNotsupp,
                };
            }
        };

        // If the component is of zero length, NFS4ERR_INVAL will be returned.
        // The component is also subject to the normal UTF-8, character support,
        // and name checks.  See Section 12.7 for further discussion.
        if file.is_empty() {
            error!("Empty file name");
            return NfsOpResponse {
                request,
                result: None,
                status: NfsStat4::Nfs4errInval,
            };
        }

        match &self.openhow {
            OpenFlag4::Open4Nocreate => {
                // Open a file for reading
                self.open_for_reading(filehandle, file, request).await
            }
            OpenFlag4::How(how) => {
                // Open a file for writing
                self.open_for_writing(filehandle, file, how, request).await
            }
        }
    }
}
