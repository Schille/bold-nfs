#![doc = include_str!("../README.md")]

pub mod server;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use bold_proto::onc_rpc::{auth::AuthFlavor, AcceptedReply, ReplyBody};
use bold_proto::onc_rpc::{AcceptedStatus, MessageType};
use bold_proto::{RpcMessage, XDRProtoCodec};
use futures::SinkExt;
use server::clientmanager::ClientManagerHandle;
use server::filemanager::{FileManager, FileManagerHandle};
use tokio::net::TcpListener;
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{error, info, span, trace, Level};
pub use vfs;
pub use vfs::VfsPath;

use crate::server::filemanager::FilehandleDb;
use crate::server::request::NfsRequest;
use crate::server::{NFSService, NfsProtoImpl};

pub struct NFSServer {
    /// The listining address of the server
    bind: String,
    /// The root of this NFS file system
    root: VfsPath,
    /// NFSv4.0 service
    service_0: Option<server::nfs40::NFS40Server>,
    /// The time the server was started
    boot_time: u64,
    // ToDo: add more minor version support
}

impl NFSServer {
    // This method will help users to discover the builder
    pub fn builder(root: VfsPath) -> ServerBuilder {
        ServerBuilder::new(root)
    }

    /// Start the NFS server, serve forever
    /// This starts a tokio runtime and serves the NFS requests
    pub fn start(&self) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let listener = TcpListener::bind(self.bind.clone()).await.unwrap();
                info!(%self.bind, "Server listening");

                // start the client manager and file manager
                // configs go here
                let client_manager_handle = ClientManagerHandle::new();
                // let file_manager_handle = FileManagerHandle::new(self.root.clone(), None);
                let filehandle_manager =
                    Arc::new(RwLock::new(FileManager::new(self.root.clone(), None)));

                loop {
                    match listener.accept().await {
                        Ok((stream, addr)) => {
                            let _ = stream.set_nodelay(true);
                            info!(%addr, "Client connected");
                            let span = span!(Level::TRACE, "client", %addr);
                            let _enter = span.enter();
                            // Reading NFS RPC messages over record marking codec
                            let mut nfs_transport = Framed::new(stream, XDRProtoCodec::new());
                            // clone NFS server to move into the pipeline and actor connects with shared state
                            // a per-client based filehandle cache
                            let mut writehandle_cache = HashMap::new();

                            loop {
                                let msg = nfs_transport.next().await;
                                match msg {
                                    Some(Ok(msg)) => {
                                        // create a NFS request
                                        let request = NfsRequest::new(
                                            addr.to_string(),
                                            client_manager_handle.clone(),
                                            self.boot_time,
                                            filehandle_manager.clone(),
                                            Some(&mut writehandle_cache),
                                        );
                                        // ToDo implement and select correct version of NFS protocol, this services all with minor version 0
                                        let nfs_protocol = self.service_0.as_ref().unwrap();
                                        let service = NFSService::new(nfs_protocol.clone());

                                        let resp = service.call(msg, request).await;
                                        match nfs_transport.send(resp).await {
                                            Ok(_) => {
                                                trace!("response sent");
                                            }
                                            Err(e) => {
                                                error!("couldn't send response: {:?}", e);
                                                break;
                                            }
                                        }
                                    }
                                    Some(Err(e)) => {
                                        error!("couldn't get message: {:?}", e);
                                        let message_type = MessageType::Reply(ReplyBody::Accepted(
                                            AcceptedReply::new(
                                                AuthFlavor::AuthNone(None),
                                                AcceptedStatus::GarbageArgs,
                                            ),
                                        ));
                                        let resp = Box::new(RpcMessage::new(0, message_type));
                                        match nfs_transport.send(resp).await {
                                            Ok(_) => {
                                                trace!("response sent");
                                            }
                                            Err(e) => {
                                                error!("couldn't send response: {:?}", e);
                                                break;
                                            }
                                        }
                                    }
                                    None => {
                                        // client closed connection
                                        info!(%addr, "Client disconnected");
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => error!("couldn't get client: {:?}", e),
                    }
                }
            });
    }
}

pub struct ServerBuilder {
    /// The listining address of the server
    bind: String,
    /// The root of this NFS file system
    root: VfsPath,
}

impl ServerBuilder {
    pub fn new(root: VfsPath) -> Self {
        ServerBuilder {
            bind: "127.0.0.1:11112".to_string(),
            root,
        }
    }

    pub fn bind(&mut self, bind: &str) -> &mut Self {
        self.bind = bind.to_string();
        self
    }

    pub fn build(&self) -> NFSServer {
        // set the boot time to now
        let boot_time = std::time::UNIX_EPOCH.elapsed().unwrap().as_secs();
        NFSServer {
            bind: self.bind.clone(),
            root: self.root.clone(),
            service_0: Some(server::nfs40::NFS40Server::new()),
            boot_time,
        }
    }
}

#[cfg(test)]
mod test_utils {
    use std::sync::{Arc, RwLock};

    use crate::server::{
        clientmanager::ClientManagerHandle,
        filemanager::{FileManager, FileManagerHandle, FilehandleDb, FilehandleVfs},
        request::NfsRequest,
    };
    use bold_proto::nfs4_proto::xdr::{
        cb_client4, clientaddr4, nfs_client_id4, verifier4, SETCLIENTID4args,
    };
    use bytes::Bytes;
    use vfs::{MemoryFS, VfsPath};

    pub fn create_dummyfs() -> VfsPath {
        let root: VfsPath = MemoryFS::new().into();
        root.create_dir_all().unwrap();
        root
    }

    pub fn create_fake_fs() -> VfsPath {
        let root: VfsPath = MemoryFS::new().into();
        let file1 = root.join("file1.txt").unwrap();
        file1
            .create_file()
            .unwrap()
            .write_all(b"Hello, World!")
            .unwrap();

        let file1 = root.join("file1.txt").unwrap();
        file1
            .create_file()
            .unwrap()
            .write_all(b"Hello, loooooooong world!")
            .unwrap();

        let dir1 = root.join("dir1").unwrap();
        dir1.create_dir_all().unwrap();

        let file2 = dir1.join("file2.txt").unwrap();
        file2
            .create_file()
            .unwrap()
            .write_all(b"Hello, file2!")
            .unwrap();

        root
    }

    pub fn create_client(verifier: [u8; 8], id: String) -> SETCLIENTID4args<Bytes> {
        SETCLIENTID4args {
            client: nfs_client_id4 {
                verifier: verifier4(Bytes::copy_from_slice(&verifier)),
                id: Bytes::from(id),
            },
            callback: cb_client4 {
                cb_program: 0,
                cb_location: clientaddr4 {
                    r_netid: "tcp".to_string(),
                    r_addr: "127.0.0.1.149.18".to_string(),
                },
            },
            callback_ident: 1,
        }
    }

    pub async fn create_nfs40_server(root: Option<VfsPath>) -> NfsRequest<'static> {
        let root = if root.is_none() {
            create_dummyfs()
        } else {
            root.unwrap()
        };

        let client_mananger_handle = ClientManagerHandle::new();
        // let file_mananger_handle = FileManagerHandle::new(root, None);
        let filehandle_manager = Arc::new(RwLock::new(FileManager::new(root, None)));

        NfsRequest::new(
            "127.0.0.1:12345".to_owned(),
            client_mananger_handle,
            0_u64,
            filehandle_manager,
            None,
        )
    }
}
