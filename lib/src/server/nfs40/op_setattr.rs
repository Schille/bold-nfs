
use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    nfs40::NfsStat4, operation::NfsOperation, request::NfsRequest,
    response::NfsOpResponse,
};

use bold_proto::nfs4_proto::{
    Attrlist4, FileAttr, NfsResOp4, SetAttr4args, SetAttr4res,
};


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
                NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opsetattr(SetAttr4res {
                        status: NfsStat4::Nfs4errStale,
                        attrsset: Attrlist4::<FileAttr>::new(None),
                    })),
                    status: NfsStat4::Nfs4errStale,
                }
            }
            Some(filehandle) => {
                let attrsset = if !self.obj_attributes.attrmask.is_empty() {
                    let attrsset = request.file_manager().set_attr(&filehandle, &self.obj_attributes.attr_vals);
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
                    attrsset
                } else {
                    Attrlist4::<FileAttr>::new(None)
                };

                NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opsetattr(SetAttr4res {
                        status: NfsStat4::Nfs4Ok,
                        attrsset,
                    })),
                    status: NfsStat4::Nfs4Ok,
                }
            }
        }
    }
}
