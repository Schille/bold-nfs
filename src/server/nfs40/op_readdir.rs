use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    filemanager::{GetFilehandleAttrsRequest, GetFilehandleRequest},
    operation::NfsOperation,
    request::NfsRequest,
    response::NfsOpResponse,
};

use super::{
    DirList4, Entry4, Fattr4, NfsResOp4, NfsStat4, ReadDir4res, ReadDir4resok, Readdir4args,
};

#[async_trait]
impl NfsOperation for Readdir4args {
    async fn execute(&self, request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 26: READDIR - Read Directory {:?}, with request {:?}",
            self, request
        );
        let current_fh = request.current_filehandle().await;
        let dir_fh = match current_fh {
            Some(filehandle) => filehandle,
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    result: None,
                    status: NfsStat4::Nfs4errServerfault,
                };
            }
        };
        let dir = dir_fh.file.read_dir().unwrap();

        let mut fnames = Vec::new();
        let mut filehandles = Vec::new();
        let dircount: usize = self.dircount as usize;
        let maxcount: usize = self.maxcount as usize;
        let mut maxcount_actual: usize = 128;
        let mut dircount_actual = 0;
        // get a list of filenames and filehandles
        for (i, entry) in dir.enumerate() {
            let name = entry.filename();
            fnames.push(name.clone());
            // if the cookie value is progressed, we add only subsequent filehandles
            if i >= self.cookie as usize {
                // this is a poor man's estimation of the XRD outputs bytes, must be improved
                // we need to know the definitve size of the output of the XDR message here, but how?
                dircount_actual = dircount_actual + 8 + name.len() + 5;
                maxcount_actual += 200;
                if dircount == 0 || (dircount > dircount_actual && maxcount > maxcount_actual) {
                    let filehandle = request
                        .file_manager()
                        .fmanager
                        .send(GetFilehandleRequest {
                            path: Some(entry.as_str().to_string()),
                            filehandle: None,
                        })
                        .await;
                    match filehandle {
                        Err(_e) => {
                            error!("None filehandle");
                            return NfsOpResponse {
                                request,
                                result: None,
                                status: NfsStat4::Nfs4errServerfault,
                            };
                        }
                        Ok(filehandle) => {
                            filehandles.push((i + 1, filehandle));
                        }
                    }
                }
            }
        }

        // get a seed of this directory, concat all files names
        let seed: String = fnames
            .iter()
            .flat_map(|s| s.as_str().chars().collect::<Vec<_>>())
            .collect();
        // take only every nth char to create a cookie verifier
        let mut cookieverf = seed
            .as_bytes()
            .iter()
            .step_by(seed.len() / 8 + 1)
            .copied()
            .collect::<Vec<_>>();
        if self.cookie != 0 && cookieverf != self.cookieverf {
            error!("Nfs4errNotSame");
            return NfsOpResponse {
                request,
                result: None,
                status: NfsStat4::Nfs4errNotSame,
            };
        }

        // if this directory is empty, we can't create a cookie verifier based on the dir contents
        // setting it to a default value
        if cookieverf.is_empty() {
            cookieverf = [0u8; 8].to_vec();
        } else if cookieverf.len() < 8 {
            let mut diff = 8 - cookieverf.len();
            while diff > 0 {
                cookieverf.push(0);
                diff -= 1;
            }
        }

        let mut tnextentry = None;
        let mut added_entries = 0;
        for (cookie, fh) in filehandles.into_iter().rev() {
            let resp = request
                .file_manager()
                .fmanager
                .send(GetFilehandleAttrsRequest {
                    filehandle_id: fh.id.clone(),
                    attrs_request: self.attr_request.clone(),
                })
                .await;
            let (answer_attrs, attrs) = match resp {
                Ok(inner) => *inner,
                Err(e) => {
                    error!("Err {:?}", e);
                    return NfsOpResponse {
                        request,
                        result: None,
                        status: NfsStat4::Nfs4errServerfault,
                    };
                }
            };

            let entry = Entry4 {
                name: fh.file.filename(),
                cookie: cookie as u64,
                attrs: Fattr4 {
                    attrmask: answer_attrs,
                    attr_vals: attrs,
                },
                nextentry: if tnextentry.is_some() {
                    Some(Box::new(tnextentry.unwrap()))
                } else {
                    None
                },
            };
            added_entries += 1;
            tnextentry = Some(entry);
        }
        let eof = {
            if tnextentry.is_some()
                && (tnextentry.clone().unwrap().cookie + added_entries) >= fnames.len() as u64
            {
                true
            } else {
                tnextentry.is_none()
            }
        };

        NfsOpResponse {
            request,
            result: Some(NfsResOp4::Opreaddir(ReadDir4res::Resok4(ReadDir4resok {
                reply: DirList4 {
                    // len: if tnextentry.is_some() { 1 } else { 0 },
                    entries: tnextentry.clone(),
                    eof,
                },
                cookieverf: cookieverf.as_slice().try_into().unwrap(),
            }))),
            status: NfsStat4::Nfs4Ok,
        }
    }
}
