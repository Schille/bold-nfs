use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use futures::stream::Collect;
use vfs::async_vfs::AsyncVfsPath;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tracing::{debug, error};

use crate::server::{operation::NfsOperation, request::NfsRequest, response::NfsOpResponse};

use bold_proto::nfs4_proto::xdr::{
    component4, dirlist4, entry4, fattr4, nfs_cookie4, nfs_resop4, nfsstat4, utf8str_cs, verifier4,
    READDIR4args, READDIR4res, READDIR4resok,
};

#[async_trait]
impl NfsOperation for READDIR4args<Bytes> {
    async fn execute<'a>(&self, request: NfsRequest<'a>) -> NfsOpResponse<'a> {
        debug!(
            "Operation 26: READDIR - Read Directory {:?}, with request {:?}",
            self, request
        );

        let current_fh = request.current_filehandle_vfs().await;
        let dir_fh = match current_fh {
            Some(filehandle) => filehandle,
            None => {
                error!("None filehandle");
                return NfsOpResponse {
                    request,
                    result: None,
                    status: nfsstat4::NFS4ERR_NOFILEHANDLE,
                };
            }
        };

        if !dir_fh.file.is_dir().await.unwrap_or(false) {
            return NfsOpResponse {
                request,
                result: None,
                status: nfsstat4::NFS4ERR_NOTDIR,
            };
        }


        let dir_entries: Vec<AsyncVfsPath> = match dir_fh.file.read_dir().await {
            Ok(dir) => dir.collect().await,
            Err(e) => {
                error!("Failed to read directory: {:?}", e);
                return NfsOpResponse {
                    request,
                    result: None,
                    status: nfsstat4::NFS4ERR_IO,
                };
            }
        };

        let mut all_entries = Vec::new();
        let mut hasher = DefaultHasher::new();

        for entry in dir_entries {
            let name = entry.filename();
            name.hash(&mut hasher);
            all_entries.push(entry);
        }

        let verf_hash = hasher.finish();
        let current_verifier = verf_hash.to_be_bytes();

        if self.cookie.0 != 0 && current_verifier != self.cookieverf.0.as_ref() {
            return NfsOpResponse {
                request,
                result: None,
                status: nfsstat4::NFS4ERR_NOT_SAME,
            };
        }

        let mut entries_to_return = Vec::new();
        let mut dircount_consumed = 0;
        // Basic overhead for READDIR4resok: status (4) + verifier (8) + eof (4) + entries_follow (4)
        // But maxcount is for the entire response. Compound overhead is also there.
        // Let's use a conservative estimate for maxcount.
        let mut maxcount_consumed = 64;

        let mut eof = true;

        for (i, entry) in all_entries.into_iter().enumerate() {
            let entry_cookie = (i + 3) as u64;
            if entry_cookie <= self.cookie.0 {
                continue;
            }

            let name = entry.filename();

            // Estimate entry size
            // name: 4 (len) + padded name
            let name_len_padded = (name.len() + 3) & !3;
            // entry overhead: name_len_padded + 4 (name len) + 8 (cookie) + 4 (nextentry flag)
            let entry_info_size = 4 + name_len_padded + 8 + 4;

            // We'll need attributes too
            let file_manager = request.file_manager.clone();
            let fh = {
                let rmanager = file_manager.read().await;
                rmanager.get_filehandle_by_path(&entry.as_str().to_string())
            };

            let fh = match fh {
                Some(fh) => fh,
                None => {
                    let mut wmanager = file_manager.write().await;
                    wmanager.get_filehandle(&entry).await
                }
            };

            let attr_resp = {
                let rmanager = file_manager.read().await;
                rmanager.filehandle_attrs(self.attr_request.clone().into(), &fh).await
            };

            let (answer_attrs, attrs) = match attr_resp {
                Some(inner) => inner,
                None => {
                    return NfsOpResponse {
                        request,
                        result: None,
                        status: nfsstat4::NFS4ERR_SERVERFAULT,
                    };
                }
            };

            let attr_vals_len = attrs.len();
            let attr_vals_padded = (attr_vals_len + 3) & !3;
            let attr_size = 4 + (answer_attrs.len() * 4) + 4 + attr_vals_padded;

            let total_entry_size = entry_info_size + attr_size;

            if (self.dircount.0 > 0
                && dircount_consumed + total_entry_size > self.dircount.0 as usize)
                || (maxcount_consumed + total_entry_size > self.maxcount.0 as usize)
            {
                if entries_to_return.is_empty() {
                    // RFC 7530: If the maxcount is too small to return even one entry,
                    // the server should return as much as possible, or an error.
                    // Usually we should return at least one if it fits in a reasonable buffer,
                    // but here we strictly follow the limits.
                    // If even one doesn't fit, we might need to return NFS4ERR_TOOSMALL?
                    // Actually, READDIR doesn't have TOOSMALL.
                    // Let's just break and return empty if it's really that small.
                }
                eof = false;
                break;
            }

            dircount_consumed += total_entry_size;
            maxcount_consumed += total_entry_size;

            entries_to_return.push(entry4 {
                name: component4(utf8str_cs(name)),
                cookie: nfs_cookie4(entry_cookie),
                attrs: fattr4 {
                    attrmask: answer_attrs.into(),
                    attr_vals: attrs.into(),
                },
                nextentry: None,
            });
        }

        // Build the linked list of entries in reverse
        let mut tnextentry: Option<Box<entry4<Bytes>>> = None;
        for mut entry in entries_to_return.into_iter().rev() {
            entry.nextentry = tnextentry;
            tnextentry = Some(Box::new(entry));
        }

        NfsOpResponse {
            request,
            result: Some(nfs_resop4::OP_READDIR(READDIR4res::NFS4_OK(
                READDIR4resok {
                    reply: dirlist4 {
                        entries: tnextentry,
                        eof,
                    },
                    cookieverf: verifier4(Bytes::from_owner(current_verifier)),
                },
            ))),
            status: nfsstat4::NFS4_OK,
        }
    }
}

#[cfg(test)]
mod integration_tests {

    use bold_proto::{
        nfs4_proto::xdr::{count4, nfs_cookie4, verifier4},
        nfs4_utils::FileAttr,
    };
    use bytes::Bytes;
    use tracing_test::traced_test;

    use crate::{
        server::{
            nfs40::{nfs_resop4, nfsstat4, PUTFH4args, READDIR4args, READDIR4res},
            operation::NfsOperation,
        },
        test_utils::{create_fake_fs, create_nfs40_server},
    };

    #[tokio::test]
    #[traced_test]
    async fn test_read_directory() {
        // dummy fs, empty
        let request = create_nfs40_server(None).await;
        let file_manager = request.file_manager.clone();
        let manager = file_manager.read().await;
        let fh = manager.get_filehandle_by_path(&String::from("/"));

        let putfh_args = PUTFH4args {
            object: fh.unwrap().id,
        };
        drop(manager);
        let putfh_request = putfh_args.execute(request).await;

        let readdir_args = READDIR4args {
            cookie: nfs_cookie4(0),
            cookieverf: verifier4(Bytes::from_owner([0u8; 8])),
            dircount: count4(262122),
            maxcount: count4(1048488),
            attr_request: vec![FileAttr::Type, FileAttr::Change, FileAttr::Size].into(),
        };

        let readdir_response = readdir_args.execute(putfh_request.request).await;
        assert_eq!(readdir_response.status, nfsstat4::NFS4_OK);
        match readdir_response.result.unwrap() {
            nfs_resop4::OP_READDIR(READDIR4res::NFS4_OK(res)) => {
                assert_eq!(res.reply.entries, None);
                assert!(res.reply.eof);
                assert_eq!(res.cookieverf.0.len(), 8);
            }
            _ => panic!("Expected Resok4"),
        }

        // a more filled directory, still eof = true

        let request = create_nfs40_server(Some(create_fake_fs().await)).await;
        let file_manager = request.file_manager.clone();
        let manager = file_manager.read().await;
        let fh = manager.get_filehandle_by_path(&String::from("/"));

        let putfh_args = PUTFH4args {
            object: fh.unwrap().id,
        };
        drop(manager);
        let putfh_request = putfh_args.execute(request).await;

        let readdir_args = READDIR4args {
            cookie: nfs_cookie4(0),
            cookieverf: verifier4(Bytes::from_owner([0u8; 8])),
            dircount: count4(262122),
            maxcount: count4(1048488),
            attr_request: vec![FileAttr::Type].into(),
        };

        let readdir_response = readdir_args.execute(putfh_request.request).await;
        assert_eq!(readdir_response.status, nfsstat4::NFS4_OK);
        let result = readdir_response.result.unwrap();
        match result {
            nfs_resop4::OP_READDIR(READDIR4res::NFS4_OK(res)) => {
                assert_eq!(res.cookieverf.0.len(), 8);
                let mut current = res.reply.entries;
                let mut count = 0;
                while let Some(entry) = current {
                    count += 1;
                    assert!(entry.cookie.0 >= 3);
                    current = entry.nextentry;
                }
                assert_eq!(count, 2); // fake_fs has file1.txt and dir1
                assert!(res.reply.eof);
            }
            _ => panic!("Expected Resok4"),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn test_read_directory_with_cookie() {
        let request = create_nfs40_server(Some(create_fake_fs().await)).await;
        let file_manager = request.file_manager.clone();
        let manager = file_manager.read().unwrap();
        let fh = manager.get_filehandle_by_path(&String::from("/"));
        let putfh_args = PUTFH4args {
            object: fh.unwrap().id,
        };
        drop(manager);
        let putfh_request = putfh_args.execute(request).await;

        // Get the first entry to get the cookie verifier and cookie
        let readdir_args = READDIR4args {
            cookie: nfs_cookie4(0),
            cookieverf: verifier4(Bytes::from_owner([0u8; 8])),
            dircount: count4(262122),
            maxcount: count4(1048488),
            attr_request: vec![FileAttr::Type].into(),
        };

        let readdir_response = readdir_args.execute(putfh_request.request).await;
        let (verf, cookie) = match readdir_response.result.unwrap() {
            nfs_resop4::OP_READDIR(READDIR4res::NFS4_OK(res)) => {
                (res.cookieverf, res.reply.entries.unwrap().cookie)
            }
            _ => panic!("Expected Resok4"),
        };

        // Now request starting from the first entry's cookie
        let readdir_args_2 = READDIR4args {
            cookie: cookie.clone(),
            cookieverf: verf,
            dircount: count4(262122),
            maxcount: count4(1048488),
            attr_request: vec![FileAttr::Type].into(),
        };

        let readdir_response_2 = readdir_args_2.execute(readdir_response.request).await;
        assert_eq!(readdir_response_2.status, nfsstat4::NFS4_OK);
        match readdir_response_2.result.unwrap() {
            nfs_resop4::OP_READDIR(READDIR4res::NFS4_OK(res)) => {
                let mut count = 0;
                let mut current = res.reply.entries;
                while let Some(entry) = current {
                    count += 1;
                    assert!(entry.cookie.0 > cookie.0);
                    current = entry.nextentry;
                }
                assert_eq!(count, 1);
                assert!(res.reply.eof);
            }
            _ => panic!("Expected Resok4"),
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn test_read_directory_limits() {
        let request = create_nfs40_server(Some(create_fake_fs().await)).await;
        let file_manager = request.file_manager.clone();
        let manager = file_manager.read().unwrap();
        let fh = manager.get_filehandle_by_path(&String::from("/"));
        let putfh_args = PUTFH4args {
            object: fh.unwrap().id,
        };
        drop(manager);
        let putfh_request = putfh_args.execute(request).await;

        // Use a very small dircount
        let readdir_args = READDIR4args {
            cookie: nfs_cookie4(0),
            cookieverf: verifier4(Bytes::from_owner([0u8; 8])),
            dircount: count4(40), // Small enough to hopefully only allow 1 entry
            maxcount: count4(1048488),
            attr_request: vec![FileAttr::Type].into(),
        };

        let readdir_response = readdir_args.execute(putfh_request.request).await;
        assert_eq!(readdir_response.status, nfsstat4::NFS4_OK);
        match readdir_response.result.unwrap() {
            nfs_resop4::OP_READDIR(READDIR4res::NFS4_OK(res)) => {
                let mut count = 0;
                let mut current = res.reply.entries;
                while let Some(entry) = current {
                    count += 1;
                    current = entry.nextentry;
                }
                assert!(count < 2);
                assert!(!res.reply.eof);
            }
            _ => panic!("Expected Resok4"),
        }
    }
}
