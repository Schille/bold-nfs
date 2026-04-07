#![doc = include_str!("../README.md")]

pub mod server;

use async_lock::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use bold_proto::onc_rpc::{auth::AuthFlavor, AcceptedReply, ReplyBody};
use bold_proto::onc_rpc::{AcceptedStatus, MessageType};
use bold_proto::RpcMessage;
use monoio::buf::{IoBuf, IoBufMut, Slice};
use monoio::io::{AsyncReadRent, AsyncReadRentExt, AsyncWriteRent, AsyncWriteRentExt};
use monoio::net::{TcpListener, TcpStream};
use server::clientmanager::ClientManagerHandle;
use server::filemanager::FileManager;
use tracing::{debug, error, info, span, Instrument, Level};
pub use vfs::async_vfs;
pub use vfs::async_vfs::AsyncVfsPath;

use crate::server::request::NfsRequest;
use crate::server::{NFSService, NfsProtoImpl};

use bytes::Bytes;

async fn read_exact<T: IoBufMut + IoBuf>(
    stream: &mut TcpStream,
    mut buf: T,
    n: usize,
) -> std::io::Result<T> {
    let mut read = 0;
    while read < n {
        let (res, slice) = stream.read(buf.slice_mut(read..n)).await;
        let m = res?;
        if m == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "EOF",
            ));
        }
        read += m;
        buf = slice.into_inner();
    }
    Ok(buf)
}

async fn read_exact_offset<T: IoBufMut + IoBuf>(
    stream: &mut TcpStream,
    mut buf: T,
    start: usize,
    end: usize,
) -> std::io::Result<T> {
    let mut read = start;
    while read < end {
        let (res, slice) = stream.read(buf.slice_mut(read..end)).await;
        let m = res?;
        if m == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "EOF",
            ));
        }
        read += m;
        buf = slice.into_inner();
    }
    Ok(buf)
}

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
    /// This starts a monoio runtime and serves the NFS requests
    pub fn start(&self) {
        let mut rt = monoio::RuntimeBuilder::<monoio::IoUringDriver>::new()
            .with_entries(256)
            .build()
            .unwrap();
        rt.block_on(async {
            let addr: std::net::SocketAddr = self.bind.parse().unwrap();
            let listener = TcpListener::bind(addr).unwrap();
            info!(%self.bind, "Server listening (monoio)");

            // start the client manager and file manager
            // configs go here
            let client_manager_handle = ClientManagerHandle::new();
            // let file_manager_handle = FileManagerHandle::new(self.root.clone(), None);
            let filehandle_manager =
                Arc::new(RwLock::new(FileManager::new(self.root.clone(), None).await));

            loop {
                match listener.accept().await {
                    Ok((mut stream, addr)) => {
                        let client_manager_handle = client_manager_handle.clone();
                        let filehandle_manager = filehandle_manager.clone();
                        let boot_time = self.boot_time;
                        let nfs_protocol = self.service_0.as_ref().unwrap().clone();

                        monoio::spawn(async move {
                            // let _ = stream.set_nodelay(true);
                            info!(%addr, "Client connected");

                            // Rent buffers
                            let mut read_buf = Some(Vec::with_capacity(64 * 1024));
                            let mut write_buf = Some(Vec::with_capacity(64 * 1024));

                            // clone NFS server to move into the pipeline and actor connects with shared state
                            // a per-client based filehandle cache
                            let mut writehandle_cache = HashMap::new();
                            let service = NFSService::new(nfs_protocol);

                            loop {
                                // Read header
                                let mut rb = read_buf.take().unwrap();
                                if rb.len() < 4 {
                                    rb.resize(4, 0);
                                }
                                match read_exact(&mut stream, rb, 4).await {
                                    Ok(b) => rb = b,
                                    Err(_) => return, // Connection closed or error
                                }

                                let header = u32::from_be_bytes([
                                    rb[0],
                                    rb[1],
                                    rb[2],
                                    rb[3],
                                ]);
                                let mut is_last = (header & (1 << 31)) != 0;
                                let mut len = (header & !(1 << 31)) as usize;

                                // Total message length in buffer (including all headers)
                                let mut total_buffer_len = 4 + len;
                                if rb.len() < total_buffer_len {
                                    rb.resize(total_buffer_len, 0);
                                }

                                match read_exact_offset(&mut stream, rb, 4, total_buffer_len).await
                                {
                                    Ok(b) => rb = b,
                                    Err(_) => return,
                                }

                                while !is_last {
                                    // Read next header
                                    let next_header_offset = total_buffer_len;
                                    total_buffer_len += 4;
                                    if rb.len() < total_buffer_len {
                                        rb.resize(total_buffer_len, 0);
                                    }
                                    match read_exact_offset(
                                        &mut stream,
                                        rb,
                                        next_header_offset,
                                        total_buffer_len,
                                    )
                                    .await
                                    {
                                        Ok(b) => rb = b,
                                        Err(_) => return,
                                    }

                                    let header = u32::from_be_bytes([
                                        rb[next_header_offset],
                                        rb[next_header_offset + 1],
                                        rb[next_header_offset + 2],
                                        rb[next_header_offset + 3],
                                    ]);
                                    is_last = (header & (1 << 31)) != 0;
                                    len = (header & !(1 << 31)) as usize;

                                    let next_fragment_offset = total_buffer_len;
                                    total_buffer_len += len;
                                    if rb.len() < total_buffer_len {
                                        rb.resize(total_buffer_len, 0);
                                    }
                                    match read_exact_offset(
                                        &mut stream,
                                        rb,
                                        next_fragment_offset,
                                        total_buffer_len,
                                    )
                                    .await
                                    {
                                        Ok(b) => rb = b,
                                        Err(_) => return,
                                    }
                                }


                                // Now we have the full message in rb[0..total_buffer_len]
                                let message_data = Bytes::copy_from_slice(&rb[..total_buffer_len]);
                                let msg_res = RpcMessage::try_from(message_data);
                                read_buf = Some(rb); // Put it back

                                match msg_res {
                                    Ok(msg) => {
                                        debug!(xid = %msg.xid(), "Received request");
                                        let request = NfsRequest::new(
                                            addr.to_string(),
                                            client_manager_handle.clone(),
                                            boot_time,
                                            filehandle_manager.clone(),
                                            Some(&mut writehandle_cache),
                                        );

                                        let resp = service.call(&Box::new(msg), request).await;
                                        debug!(xid = %resp.xid(), "Processed request, sending response");

                                        // Encode response
                                        let mut wb = write_buf.take().unwrap();
                                        wb.clear();
                                        // wb.resize(4, 0);
                                        if let Err(e) = resp.serialise_into(&mut wb) {
                                            error!("Failed to serialise response: {:?}", e);
                                            return;
                                        }
                                        // let resp_len = (wb.len() - 4) as u32;
                                        // let resp_header = (resp_len | (1 << 31)).to_be_bytes();
                                        // wb[0..4].copy_from_slice(&resp_header);

                                        let (res, b) = stream.write_all(wb).await;
                                        write_buf = Some(b);
                                        if res.is_err() {
                                            return;
                                        }
                                    }
                                    Err(e) => {
                                        error!("couldn't get message: {:?}", e);
                                        let message_type = MessageType::<Bytes, Bytes>::Reply(ReplyBody::Accepted(
                                            AcceptedReply::new(
                                                AuthFlavor::AuthNone(None),
                                                AcceptedStatus::GarbageArgs,
                                            ),
                                        ));
                                        let resp = Box::new(RpcMessage::new(0, message_type));
                                        let mut wb = write_buf.take().unwrap();
                                        wb.clear();
                                        wb.resize(4, 0);
                                        let _ = resp.serialise_into(&mut wb);
                                        let resp_len = (wb.len() - 4) as u32;
                                        let resp_header = (resp_len | (1 << 31)).to_be_bytes();
                                        wb[0..4].copy_from_slice(&resp_header);

                                        let (res, b) = stream.write_all(wb).await;
                                        write_buf = Some(b);
                                        if res.is_err() {
                                            return;
                                        }
                                    }
                                }
                            }


                            info!(%addr, "Client disconnected");
                        });
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
