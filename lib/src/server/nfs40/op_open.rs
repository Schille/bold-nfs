use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tracing::{debug, error};

use crate::server::{
    filemanager::FilehandleVfs,
    nfs40::{change_info4, open_delegation4, OPEN4res, OPEN4resok, OPEN4_RESULT_CONFIRM},
    operation::NfsOperation,
    request::NfsRequest,
    response::NfsOpResponse,
};

use bold_proto::nfs4_proto::xdr::{
    bitmap4, changeid4, createhow4, nfs_resop4, nfsstat4, open_claim4, openflag4, stateid4,
    OPEN4args, Serialisable,
};

async fn open_for_reading<'a>(
    args: &OPEN4args<Bytes>,
    file: &String,
    mut request: NfsRequest<'a>,
) -> NfsOpResponse<'a> {
    let filehandle = &request.filehandle_vfs;
    let path = &filehandle.as_ref().unwrap().path;

    let fh_path = {
        if path == "/" {
            format!("{}{}", path, file)
        } else {
            format!("{}/{}", path, file)
        }
    };

    debug!("open_for_reading {:?}", fh_path);

    let file_manager = request.file_manager.clone();

    let filehandle = {
        let manager = file_manager.read().await;
        manager.get_filehandle_by_path(&fh_path)
    };

    let mut filehandle = if let Some(filehandle_vfs) = filehandle {
        filehandle_vfs
    } else {
        let mut manager: async_lock::RwLockWriteGuard<'_, crate::server::filemanager::FileManager> =
            file_manager.write().await;
        let req_file = manager.root.join(&fh_path).unwrap();
        debug!("open_for_reading req_file {:?}", req_file);
        if req_file.exists().await.unwrap() {
            let new_fh = manager.get_filehandle(&req_file).await;
            new_fh
        } else {
            return NfsOpResponse {
                request,
                // result: None,
                status: nfsstat4::NFS4ERR_STALE,
                buffer: Bytes::new(),
            };
        }
    };

    // check existing lock
    let filehandle = {
        let file_manager = request.file_manager.clone();
        let mut wmanager = file_manager.write().await;
        wmanager.create_shared_reservation(
            &mut filehandle,
            args.owner.clientid.0,
            args.owner.owner.clone(),
        );
        let filehandle = wmanager.attach_locks(filehandle);
        filehandle
    };

    let _ = request.set_filehandle(&filehandle.id).await;

    let result: nfs_resop4<Bytes> = nfs_resop4::OP_OPEN(OPEN4res::NFS4_OK(OPEN4resok {
        stateid: stateid4 {
            seqid: 0,
            other: Bytes::from_owner([0; 12]),
        },
        cinfo: change_info4 {
            atomic: false,
            before: changeid4(0),
            after: changeid4(0),
        },
        // OPEN4_RESULT_CONFIRM indicates that the client MUST execute an
        // OPEN_CONFIRM operation before using the open file.
        rflags: OPEN4_RESULT_CONFIRM,
        attrset: bitmap4(vec![]),
        delegation: open_delegation4::OPEN_DELEGATE_NONE,
    }));
    let mut buffer = BytesMut::new();
    result.serialize(&mut buffer);

    NfsOpResponse {
        request,
        // result: Some(result),
        status: nfsstat4::NFS4_OK,
        buffer: buffer.freeze(),
    }
}

async fn open_for_writing<'a>(
    args: &OPEN4args<Bytes>,
    filehandle: &FilehandleVfs,
    file: &String,
    how: &createhow4<Bytes>,
    mut request: NfsRequest<'a>,
) -> NfsOpResponse<'a> {
    let path = &filehandle.path;

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
        createhow4::UNCHECKED4(_fattr) => {
            let file_manager = request.file_manager.clone();
            let mut wmanager = file_manager.write().await;

            let filehandle_vfs = wmanager
                .create_file(
                    &newfile_op.unwrap(),
                    args.owner.clientid.0,
                    args.owner.owner.clone(),
                    args.share_access,
                    args.share_deny,
                )
                .await;

            match filehandle_vfs {
                Some(filehandle) => filehandle,
                None => {
                    return NfsOpResponse {
                        request,
                        // result: None,
                        status: nfsstat4::NFS4ERR_SERVERFAULT,
                        buffer: Bytes::new(),
                    };
                }
            }
        }
        createhow4::EXCLUSIVE4(_verifier) => {
            let file_manager = request.file_manager.clone();
            let mut wmanager = file_manager.write().await;

            let filehandle_vfs = wmanager
                .create_file(
                    &newfile_op.unwrap(),
                    args.owner.clientid.0,
                    args.owner.owner.clone(),
                    args.share_access,
                    args.share_deny,
                )
                .await;

            match filehandle_vfs {
                Some(filehandle) => filehandle,
                None => {
                    return NfsOpResponse {
                        request,
                        // result: None,
                        status: nfsstat4::NFS4ERR_SERVERFAULT,
                        buffer: Bytes::new(),
                    };
                }
            }
        }
        _ => {
            error!("Unsupported createhow4 {:?}", how);
            return NfsOpResponse {
                request,
                // result: None,
                status: nfsstat4::NFS4ERR_NOTSUPP,
                buffer: Bytes::new(),
            };
        }
    };

    request.set_filehandle(&filehandle.id).await;
    // we expect this filehandle to have one lock (for the shared reservation)
    let lock = &filehandle.locks[0];

    let result: nfs_resop4<Bytes> = nfs_resop4::OP_OPEN(OPEN4res::NFS4_OK(OPEN4resok {
        stateid: stateid4 {
            seqid: lock.seqid,
            other: Bytes::from_owner(lock.stateid),
        },
        cinfo: change_info4 {
            atomic: false,
            before: changeid4(0),
            after: changeid4(0),
        },
        // OPEN4_RESULT_CONFIRM indicates that the client MUST execute an
        // OPEN_CONFIRM operation before using the open file.
        rflags: OPEN4_RESULT_CONFIRM,
        attrset: bitmap4(vec![]),
        delegation: open_delegation4::OPEN_DELEGATE_NONE,
    }));
    let mut buffer = BytesMut::new();
    result.serialize(&mut buffer);

    NfsOpResponse {
        request,
        // result: Some(result),
        status: nfsstat4::NFS4_OK,
        buffer: buffer.freeze(),
    }
}

#[async_trait]
impl NfsOperation for OPEN4args<Bytes> {
    async fn execute<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        // Description: https://datatracker.ietf.org/doc/html/rfc7530#section-16.16.5
        debug!(
            "Operation 18: OPEN - Open a Regular File {:?}, with request {:?}",
            self, request
        );
        // open sets the current filehandle to the looked up filehandle
        let current_filehandle = &request.filehandle_vfs;
        let filehandle = match current_filehandle {
            Some(ref filehandle) => filehandle,
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    // result: None,
                    status: nfsstat4::NFS4ERR_FHEXPIRED,
                    buffer: Bytes::new(),
                };
            }
        };

        // If the current filehandle is not a directory, the error
        // NFS4ERR_NOTDIR will be returned.
        if !filehandle.file.is_dir().await.unwrap() {
            error!("Not a directory");
            return NfsOpResponse {
                request,
                // result: None,
                status: nfsstat4::NFS4ERR_NOTDIR,
                buffer: Bytes::new(),
            };
        }

        let file = match &self.claim {
            // CLAIM_NULL:  For the client, this is a new OPEN request, and there is
            // no previous state associated with the file for the client.
            open_claim4::CLAIM_NULL(file) => file,
            // NFS4ERR_NOTSUPP is returned if the server does not support this
            // claim type.
            _ => {
                error!("Unsupported open_claim4 {:?}", self.claim);
                return NfsOpResponse {
                    request,
                    // result: None,
                    status: nfsstat4::NFS4ERR_NOTSUPP,
                    buffer: Bytes::new(),
                };
            }
        };

        // If the component is of zero length, NFS4ERR_INVAL will be returned.
        // The component is also subject to the normal UTF-8, character support,
        // and name checks.  See Section 12.7 for further discussion.
        if file.0 .0.is_empty() {
            error!("Empty file name");
            return NfsOpResponse {
                request,
                // result: None,
                status: nfsstat4::NFS4ERR_INVAL,
                buffer: Bytes::new(),
            };
        }

        match &self.openhow {
            openflag4::OPEN4_NOCREATE => {
                // Open a file for reading
                open_for_reading(self, &file.0 .0, request).await
            }
            openflag4::default => {
                // Open a file for reading
                open_for_reading(self, &file.0 .0, request).await
            }
            openflag4::OPEN4_CREATE(how) => {
                // Open a file for writing
                open_for_writing(self, &filehandle.clone(), &file.0 .0, how, request).await
            }
        }
    }
}
