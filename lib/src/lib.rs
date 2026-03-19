#![doc = include_str!("../README.md")]

pub mod server;

use async_lock::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bold_proto::onc_rpc::{auth::AuthFlavor, AcceptedReply, ReplyBody};
use bold_proto::onc_rpc::{AcceptedStatus, MessageType};
use bold_proto::{RpcMessage, XDRProtoCodec};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use server::clientmanager::ClientManagerHandle;
use server::filemanager::FileManager;
use tokio::net::TcpListener;
use tokio::time::timeout;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, span, trace, Instrument, Level};
pub use vfs::async_vfs;
pub use vfs::async_vfs::AsyncVfsPath;

use crate::server::request::NfsRequest;
use crate::server::{NFSService, NfsProtoImpl};

pub struct NFSServer {
    /// The listining address of the server
    bind: String,
    /// The root of this NFS file system
    root: AsyncVfsPath,
    /// NFSv4.0 service
    service_0: Option<server::nfs40::NFS40Server>,
    /// The time the server was started
    boot_time: u64,
    // ToDo: add more minor version support
}

impl NFSServer {
    // This method will help users to discover the builder
    pub fn builder(root: AsyncVfsPath) -> ServerBuilder {
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
                    Arc::new(RwLock::new(FileManager::new(self.root.clone(), None).await));

                loop {
                    match listener.accept().await {
                        Ok((stream, addr)) => {
                            let client_manager_handle = client_manager_handle.clone();
                            let filehandle_manager = filehandle_manager.clone();
                            let boot_time = self.boot_time;
                            let nfs_protocol = self.service_0.as_ref().unwrap().clone();

                            tokio::spawn(async move {
                                // let _ = stream.set_nodelay(false);
                                info!(%addr, "Client connected");
                                // Reading NFS RPC messages over record marking codec
                                let nfs_transport = Framed::new(stream, XDRProtoCodec::new());
                                let (mut nfs_sink, mut nfs_stream) = nfs_transport.split();
                                let (resp_tx, mut resp_rx) = tokio::sync::mpsc::channel::<Box<RpcMessage<Bytes, Bytes>>>(100);

                                // Response writer task
                                tokio::spawn(async move {
                                    loop {

                                        match timeout(Duration::from_millis(5), resp_rx.recv()).await {
                                            Ok(Some(msg)) => {
                                                debug!("received message, now feeding in");
                                                if let Err(e) = nfs_sink.feed(msg).await {
                                                    error!("couldn't send response: {:?}", e);
                                                    break;
                                                }
                                                trace!("response sent");
                                            }
                                            Ok(None) => {
                                                trace!("channel closed");
                                            },
                                            Err(_) => {
                                                //debug!("now flusing sender buffer");
                                                let _ = nfs_sink.flush().await;
                                            },
                                        }
                                    }
                                }.instrument(span!(Level::TRACE, "writer", %addr)));

                                // clone NFS server to move into the pipeline and actor connects with shared state
                                // a per-client based filehandle cache
                                let mut writehandle_cache = HashMap::new();
                                let service = NFSService::new(nfs_protocol);

                                loop {
                                    let msg = nfs_stream.next().await;
                                    match msg {
                                        Some(Ok(msg)) => {
                                            debug!(xid = %msg.xid(), "Received request");
                                            // create a NFS request
                                            let request = NfsRequest::new(
                                                addr.to_string(),
                                                client_manager_handle.clone(),
                                                boot_time,
                                                filehandle_manager.clone(),
                                                Some(&mut writehandle_cache),
                                            );

                                            let resp = service.call(msg, request).await;
                                            debug!(xid = %resp.xid(), "Processed request, queueing response");
                                            if resp_tx.send(resp).await.is_err() {
                                                break;
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
                                            if resp_tx.send(resp).await.is_err() {
                                                break;
                                            }
                                        }
                                        None => {
                                            // client closed connection
                                            info!(%addr, "Client disconnected");
                                            break;
                                        }
                                    }
                                }
                            }.instrument(span!(Level::TRACE, "client", %addr)));
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
    root: AsyncVfsPath,
}

impl ServerBuilder {
    pub fn new(root: AsyncVfsPath) -> Self {
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
    use async_lock::RwLock;
    use std::sync::Arc;

    use crate::server::{
        clientmanager::ClientManagerHandle,
        filemanager::{FileManager, FileManagerHandle, FilehandleDb, FilehandleVfs},
        request::NfsRequest,
    };
    use bold_proto::nfs4_proto::xdr::{
        cb_client4, clientaddr4, nfs_client_id4, verifier4, SETCLIENTID4args,
    };
    use bytes::Bytes;
    use futures::AsyncWriteExt;
    use vfs::async_vfs::{AsyncMemoryFS, AsyncVfsPath};

    pub async fn create_dummyfs() -> AsyncVfsPath {
        let root: AsyncVfsPath = AsyncMemoryFS::new().into();
        root.create_dir_all().await.unwrap();
        root
    }

    pub async fn create_fake_fs() -> AsyncVfsPath {
        let root: AsyncVfsPath = AsyncMemoryFS::new().into();
        let file1 = root.join("file1.txt").unwrap();
        file1
            .create_file()
            .await
            .unwrap()
            .write_all(b"Hello, World!")
            .await
            .unwrap();

        let file1 = root.join("file1.txt").unwrap();
        file1
            .create_file()
            .await
            .unwrap()
            .write_all(b"Hello, loooooooong world!")
            .await
            .unwrap();

        let dir1 = root.join("dir1").unwrap();
        dir1.create_dir_all().await.unwrap();

        let file2 = dir1.join("file2.txt").unwrap();
        file2
            .create_file()
            .await
            .unwrap()
            .write_all(b"Hello, file2!")
            .await
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

    pub async fn create_nfs40_server(root: Option<AsyncVfsPath>) -> NfsRequest<'static> {
        let root = if root.is_none() {
            create_dummyfs().await
        } else {
            root.unwrap()
        };

        let client_mananger_handle = ClientManagerHandle::new();
        // let file_mananger_handle = FileManagerHandle::new(root, None);
        let filehandle_manager = Arc::new(RwLock::new(FileManager::new(root, None).await));

        NfsRequest::new(
            "127.0.0.1:12345".to_owned(),
            client_mananger_handle,
            0_u64,
            filehandle_manager,
            None,
        )
    }
}
