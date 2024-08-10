use actix::prelude::*;
use bold::{
    proto::NFSProtoCodec,
    server::{
        clientmanager::{ClientManager, ClientManagerHandler}, filemanager::{FileManager, FileManagerHandler}, nfs40::NFS40Server, request::NfsRequest, NFSService, NfsProtoImpl
    },
};
use futures::sink::SinkExt;
use tokio::net::TcpListener;
use tokio_stream::StreamExt;

use tokio_util::codec::Framed;
use tracing::{error, info, span, trace, Level};
use vfs::{AltrootFS, PhysicalFS, VfsPath};

#[actix::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .event_format(
            tracing_subscriber::fmt::format()
                .with_target(true)
                .with_source_location(true),
        )
        .with_max_level(Level::DEBUG)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let root: VfsPath = AltrootFS::new(VfsPath::new(PhysicalFS::new(
        std::env::current_dir().unwrap(),
    )))
    .into();

    let bind = "127.0.0.1:11112";
    let listener = TcpListener::bind(bind).await.unwrap();
    info!(%bind, "Server listening");
    // start a global Actix ClientManager actor
    let client_manager_addr = ClientManager::new().start();
    let file_manager_addr = FileManager::new(root, None).start();
    // dynamic dispatch to NFSv4.0 server implementation
    // TODO add support for multiple NFSv4 minor versions
    let nfs_protocol = NFS40Server::new(client_manager_addr.clone(), file_manager_addr.clone());
    // let nfs_server = NFSService::new(nfs_protocol);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let _ = stream.set_nodelay(true);
                info!(%addr, "Client connected");
                let _span = span!(Level::TRACE, "client");
                // Reading NFS RPC messages over record marking codec
                let mut nfs_transport = Framed::new(stream, NFSProtoCodec::new());
                // clone NFS server to move into the pipeline and actor connects with shared state
                let service = NFSService::new(nfs_protocol.clone());
                loop {
                    let msg = nfs_transport.next().await;
                    match msg {
                        Some(Ok(msg)) => {
                            // create a NFS request
                            let request = NfsRequest::new(
                                addr.to_string(),
                                ClientManagerHandler::new(client_manager_addr.clone()),
                                FileManagerHandler::new(file_manager_addr.clone()),
                            );
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
                            break;
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
}
