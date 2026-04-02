use std::hash::{Hash, Hasher};
use std::sync::LazyLock;

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use dashmap::DashMap;
use tracing::{debug, error};

use crate::server::filemanager::FilehandleVfs;
use crate::server::{
    nfs40::nfsstat4, operation::NfsOperation, request::NfsRequest, response::NfsOpResponse,
};

use bold_proto::nfs4_proto::xdr::{
    attrlist4, bitmap4, fattr4, nfs_resop4, GETATTR4args, GETATTR4res, GETATTR4resok, Serialisable,
};
use bold_proto::nfs4_utils::FileAttr;

static CACHE: LazyLock<DashMap<u64, Bytes>> = LazyLock::new(|| DashMap::new());

#[async_trait]
impl NfsOperation for GETATTR4args {
    async fn execute<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 9: GETATTR - Get Attributes {:?}, with request {:?}",
            self, request
        );
        let filehandle = &request.filehandle_vfs;
        match filehandle {
            None => {
                error!("None filehandle");
                let result: nfs_resop4<Bytes> =
                    nfs_resop4::OP_GETATTR(GETATTR4res::NFS4_OK(GETATTR4resok {
                        obj_attributes: fattr4 {
                            attrmask: bitmap4(Vec::new()),
                            attr_vals: attrlist4(Bytes::new()),
                        },
                    }));
                let mut buffer = BytesMut::new();
                result.serialize(&mut buffer);
                return NfsOpResponse {
                    request,
                    // result: Some(result),
                    status: nfsstat4::NFS4ERR_STALE,
                    buffer: buffer.freeze(),
                };
            }
            Some(filehandle) => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                self.attr_request.hash(&mut hasher);
                filehandle.id.hash(&mut hasher);
                let cache_key = hasher.finish();

                if let Some(val) = CACHE.get(&cache_key) {
                    return NfsOpResponse {
                        request,
                        status: nfsstat4::NFS4_OK,
                        buffer: val.clone(),
                    };
                }

                let attr_vec: Vec<FileAttr> = self.attr_request.clone().into();

                let file_manager = request.file_manager.clone();
                let rmanager = file_manager.read().await;

                let val = rmanager.filehandle_attrs(&attr_vec, &filehandle).await;
                let fatattr = if let Some((answer_attrs, attrs)) = val {
                    fattr4 {
                        attrmask: answer_attrs.into(),
                        attr_vals: attrs.into(),
                    }
                } else {
                    let result: nfs_resop4<Bytes> =
                        nfs_resop4::OP_GETATTR(GETATTR4res::NFS4_OK(GETATTR4resok {
                            obj_attributes: fattr4 {
                                attrmask: bitmap4(Vec::new()),
                                attr_vals: attrlist4(Bytes::new()),
                            },
                        }));
                    let mut buffer = BytesMut::new();
                    result.serialize(&mut buffer);
                    return NfsOpResponse {
                        request,
                        // result: Some(result),
                        status: nfsstat4::NFS4ERR_SERVERFAULT,
                        buffer: buffer.freeze(),
                    };
                };

                let result: nfs_resop4<Bytes> =
                    nfs_resop4::OP_GETATTR(GETATTR4res::NFS4_OK(GETATTR4resok {
                        obj_attributes: fatattr,
                    }));
                let mut buffer = BytesMut::new();
                result.serialize(&mut buffer);
                let frozen = buffer.freeze();

                CACHE.insert(cache_key, frozen.clone());

                NfsOpResponse {
                    request,
                    // result: Some(result),
                    status: nfsstat4::NFS4_OK,
                    buffer: frozen,
                }
            }
        }
    }
}

