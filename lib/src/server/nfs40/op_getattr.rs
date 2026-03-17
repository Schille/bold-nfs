use async_trait::async_trait;
use bytes::Bytes;
use tracing::{debug, error};

use crate::server::{
    nfs40::nfsstat4, operation::NfsOperation, request::NfsRequest, response::NfsOpResponse,
};

use bold_proto::nfs4_proto::xdr::{
    attrlist4, bitmap4, fattr4, nfs_resop4, GETATTR4args, GETATTR4res, GETATTR4resok,
};

#[async_trait]
impl NfsOperation for GETATTR4args {
    async fn execute<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 9: GETATTR - Get Attributes {:?}, with request {:?}",
            self, request
        );
        let filehandle = request.current_filehandle_vfs();
        match filehandle {
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    result: Some(nfs_resop4::OP_GETATTR(GETATTR4res::NFS4_OK(
                        GETATTR4resok {
                            obj_attributes: fattr4 {
                                attrmask: bitmap4(Vec::new()),
                                attr_vals: attrlist4(Bytes::new()),
                            },
                        },
                    ))),
                    status: nfsstat4::NFS4ERR_STALE,
                };
            }
            Some(filehandle) => {
                let file_manager = request.file_manager.clone();
                let rmanager = file_manager.read().unwrap();
                let attr = self.attr_request.clone();
                let resp = rmanager.filehandle_attrs(attr.into(), &filehandle);

                let (answer_attrs, attrs) = match resp {
                    Some(inner) => inner,
                    None => {
                        return NfsOpResponse {
                            request,
                            result: Some(nfs_resop4::OP_GETATTR(GETATTR4res::NFS4_OK(
                                GETATTR4resok {
                                    obj_attributes: fattr4 {
                                        attrmask: bitmap4(Vec::new()),
                                        attr_vals: attrlist4(Bytes::new()),
                                    },
                                },
                            ))),
                            status: nfsstat4::NFS4ERR_SERVERFAULT,
                        };
                    }
                };

                NfsOpResponse {
                    request,
                    result: Some(nfs_resop4::OP_GETATTR(GETATTR4res::NFS4_OK(
                        GETATTR4resok {
                            obj_attributes: fattr4 {
                                attrmask: answer_attrs.into(),
                                attr_vals: attrs.into(),
                            },
                        },
                    ))),
                    status: nfsstat4::NFS4_OK,
                }
            }
        }
    }
}

// #[cfg(test)]
// mod integration_tests {
//     use crate::{
//         server::{
//             nfs40::{
//                 fattr4, FileAttr, FileAttrValue, Fsid4, Getattr4args, NfsFtype4, nfs_resop4,
//                 NfsStat4,
//             },
//             operation::NfsOperation,
//         },
//         test_utils::create_nfs40_server,
//     };
//     use tracing_test::traced_test;

//     #[tokio::test]
//     #[traced_test]
//     async fn test_get_attr() {
//         let mut request = create_nfs40_server(None).await;
//         let fh = request.file_manager().get_root_filehandle().await;
//         let _ = request.set_filehandle_id(fh.unwrap().id).await;

//         let args1 = Getattr4args {
//             attr_request: vec![FileAttr::LeaseTime],
//         };

//         let res1 = args1.execute(request.clone()).await;
//         assert_eq!(res1.status, crate::server::nfs40::nfsstat4::NFS4_OK);
//         match res1.result {
//             Some(nfs_resop4::Opgetattr(res)) => {
//                 assert_eq!(res.status, nfsstat4::NFS4_OK);
//                 assert_eq!(
//                     res.obj_attributes,
//                     Some(fattr4 {
//                         attrmask: vec![FileAttr::LeaseTime],
//                         attr_vals: vec![FileAttrValue::LeaseTime(60)]
//                     })
//                 );
//             }
//             _ => panic!("Unexpected result"),
//         }

//         let args2 = Getattr4args {
//             attr_request: vec![
//                 FileAttr::Type,
//                 FileAttr::Change,
//                 FileAttr::Size,
//                 FileAttr::Fsid,
//                 FileAttr::Fileid,
//                 FileAttr::Mode,
//                 FileAttr::Numlinks,
//                 FileAttr::Owner,
//                 FileAttr::OwnerGroup,
//             ],
//         };

//         let res2 = args2.execute(request.clone()).await;
//         assert_eq!(res2.status, crate::server::nfs40::nfsstat4::NFS4_OK);
//         match res2.result {
//             Some(nfs_resop4::Opgetattr(res)) => {
//                 assert_eq!(res.status, nfsstat4::NFS4_OK);
//                 assert_eq!(
//                     res.obj_attributes,
//                     Some(fattr4 {
//                         attrmask: vec![
//                             FileAttr::Type,
//                             FileAttr::Change,
//                             FileAttr::Size,
//                             FileAttr::Fsid,
//                             FileAttr::Fileid,
//                             FileAttr::Mode,
//                             FileAttr::Numlinks,
//                             FileAttr::Owner,
//                             FileAttr::OwnerGroup
//                         ],
//                         attr_vals: vec![
//                             FileAttrValue::Type(NfsFtype4::Nf4dir),
//                             FileAttrValue::Change(1),
//                             FileAttrValue::Size(0),
//                             FileAttrValue::Fsid(Fsid4 {
//                                 major: 152,
//                                 minor: 152
//                             }),
//                             FileAttrValue::Fileid(3476900567878811119),
//                             FileAttrValue::Mode(292),
//                             FileAttrValue::Numlinks(1),
//                             FileAttrValue::Owner("1000".to_string()),
//                             FileAttrValue::OwnerGroup("1000".to_string())
//                         ]
//                     })
//                 );
//             }
//             _ => panic!("Unexpected result"),
//         }
//     }
// }
