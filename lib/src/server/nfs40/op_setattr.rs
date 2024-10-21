use std::vec;

use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    filemanager::Filehandle, nfs40::NfsStat4, operation::NfsOperation, request::NfsRequest,
    response::NfsOpResponse,
};

use bold_proto::nfs4_proto::{Attrlist4, FileAttrValue, NfsResOp4, SetAttr4args, SetAttr4res};

fn set_attr(filehandle: &Filehandle, attr_vals: &Attrlist4<FileAttrValue>) {
    for attr in attr_vals.iter() {
        match attr {
            FileAttrValue::Size(args) => {
                debug!("Set size to: {:?}", args);
                let mut buf = vec![0_u8; *args as usize];
                let mut file = filehandle.file.open_file().unwrap();
                let _ = file.rewind();
                file.read_exact(&mut buf).unwrap();

                let mut file = filehandle.file.append_file().unwrap();
                let _ = file.rewind();
                file.write_all(&buf).unwrap();
                file.flush().unwrap();
            }
            _ => {
                error!("Not supported set attr requested for: {:?}", attr);
            }
        }
    }
}

#[async_trait]
impl NfsOperation for SetAttr4args {
    async fn execute<'a>(&self, mut request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 34: SETATTR - Set Attributes {:?}, with request {:?}",
            self, request
        );
        let filehandle = request.current_filehandle();
        match filehandle {
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opsetattr(SetAttr4res {
                        status: NfsStat4::Nfs4errStale,
                        attrsset: Vec::new(),
                    })),
                    status: NfsStat4::Nfs4errStale,
                };
            }
            Some(filehandle) => {
                if !self.obj_attributes.attrmask.is_empty() {
                    // TODO implement set attr, check
                    set_attr(&filehandle, &self.obj_attributes.attr_vals);
                    request.drop_filehandle_from_cache(filehandle.id.clone());

                    request
                        .file_manager()
                        .touch_file(filehandle.id.clone())
                        .await;
                    match request.set_filehandle_id(filehandle.id.clone()).await {
                        Ok(fh) => {
                            request.cache_filehandle(fh);
                        }
                        Err(e) => {
                            return NfsOpResponse {
                                request,
                                result: None,
                                status: e,
                            };
                        }
                    }
                }

                NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opsetattr(SetAttr4res {
                        status: NfsStat4::Nfs4Ok,
                        attrsset: Vec::new(),
                    })),
                    status: NfsStat4::Nfs4Ok,
                }
            }
        }
    }
}
