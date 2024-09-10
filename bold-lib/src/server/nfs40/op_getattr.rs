use async_trait::async_trait;
use tracing::{debug, error};

use crate::server::{
    nfs40::NfsStat4, operation::NfsOperation, request::NfsRequest, response::NfsOpResponse,
};

use super::{Fattr4, Getattr4args, Getattr4resok, NfsResOp4};

#[async_trait]
impl NfsOperation for Getattr4args {
    async fn execute(&self, request: NfsRequest) -> NfsOpResponse {
        debug!(
            "Operation 9: GETATTR - Get Attributes {:?}, with request {:?}",
            self, request
        );
        let filehandle = request.current_filehandle_id();
        match filehandle {
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opgetattr(Getattr4resok {
                        obj_attributes: None,
                        status: NfsStat4::Nfs4errStale,
                    })),
                    status: NfsStat4::Nfs4errStale,
                };
            }
            Some(filehandle_id) => {
                let resp = request
                    .file_manager()
                    .get_filehandle_attrs(filehandle_id, self.attr_request.clone())
                    .await;
                let (answer_attrs, attrs) = match resp {
                    Ok(inner) => *inner,
                    Err(e) => {
                        error!("FileManagerError {:?}", e);
                        return NfsOpResponse {
                            request,
                            result: Some(NfsResOp4::Opgetattr(Getattr4resok {
                                obj_attributes: None,
                                status: e.nfs_error.clone(),
                            })),
                            status: e.nfs_error,
                        };
                    }
                };

                NfsOpResponse {
                    request,
                    result: Some(NfsResOp4::Opgetattr(Getattr4resok {
                        status: NfsStat4::Nfs4Ok,
                        obj_attributes: Some(Fattr4 {
                            attrmask: answer_attrs,
                            attr_vals: attrs,
                        }),
                    })),
                    status: NfsStat4::Nfs4Ok,
                }
            }
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::{
        server::{
            nfs40::{
                Fattr4, FileAttr, FileAttrValue, Fsid4, Getattr4args, NfsFtype4, NfsResOp4,
                NfsStat4,
            },
            operation::NfsOperation,
        },
        test_utils::create_nfs40_server,
    };
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn test_get_attr() {
        let mut request = create_nfs40_server(None).await;
        let fh = request.file_manager().get_root_filehandle().await;
        request.set_filehandle_id(fh.unwrap().id);

        let args1 = Getattr4args {
            attr_request: vec![FileAttr::LeaseTime],
        };

        let res1 = args1.execute(request.clone()).await;
        assert_eq!(res1.status, crate::server::nfs40::NfsStat4::Nfs4Ok);
        match res1.result {
            Some(NfsResOp4::Opgetattr(res)) => {
                assert_eq!(res.status, NfsStat4::Nfs4Ok);
                assert_eq!(
                    res.obj_attributes,
                    Some(Fattr4 {
                        attrmask: vec![FileAttr::LeaseTime],
                        attr_vals: vec![FileAttrValue::LeaseTime(60)]
                    })
                );
            }
            _ => panic!("Unexpected result"),
        }

        let args2 = Getattr4args {
            attr_request: vec![
                FileAttr::Type,
                FileAttr::Change,
                FileAttr::Size,
                FileAttr::Fsid,
                FileAttr::Fileid,
                FileAttr::Mode,
                FileAttr::Numlinks,
                FileAttr::Owner,
                FileAttr::OwnerGroup,
            ],
        };

        let res2 = args2.execute(request.clone()).await;
        assert_eq!(res2.status, crate::server::nfs40::NfsStat4::Nfs4Ok);
        match res2.result {
            Some(NfsResOp4::Opgetattr(res)) => {
                assert_eq!(res.status, NfsStat4::Nfs4Ok);
                assert_eq!(
                    res.obj_attributes,
                    Some(Fattr4 {
                        attrmask: vec![
                            FileAttr::Type,
                            FileAttr::Change,
                            FileAttr::Size,
                            FileAttr::Fsid,
                            FileAttr::Fileid,
                            FileAttr::Mode,
                            FileAttr::Numlinks,
                            FileAttr::Owner,
                            FileAttr::OwnerGroup
                        ],
                        attr_vals: vec![
                            FileAttrValue::Type(NfsFtype4::Nf4dir),
                            FileAttrValue::Change(0),
                            FileAttrValue::Size(0),
                            FileAttrValue::Fsid(Fsid4 {
                                major: 152,
                                minor: 152
                            }),
                            FileAttrValue::Fileid(3476900567878811119),
                            FileAttrValue::Mode(292),
                            FileAttrValue::Numlinks(1),
                            FileAttrValue::Owner("1000".to_string()),
                            FileAttrValue::OwnerGroup("1000".to_string())
                        ]
                    })
                );
            }
            _ => panic!("Unexpected result"),
        }
    }
}
