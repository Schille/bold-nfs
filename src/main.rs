use std::{cmp::{max, min}, path};

use nfsv4::{proto::NFSProtoCodec, server::{nfs40::NFS40Server, NFSProtoImpl, NFSService}};
use tokio::net::TcpListener;
use tokio_tower::pipeline;
use tokio_util::codec::Framed;
use tracing::{error, event, info, span, trace, Instrument, Level};
use vfs::{VfsPath, PhysicalFS, AltrootFS};


#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt().event_format(
        tracing_subscriber::fmt::format()
            .with_target(true) 
            .with_source_location(true)

    ).with_max_level(Level::DEBUG).finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let root: VfsPath = AltrootFS::new(VfsPath::new(PhysicalFS::new(std::env::current_dir().unwrap()))).into();
    

    let bind = "127.0.0.1:11112";
    let listener = TcpListener::bind(bind).await.unwrap();
    info!(%bind, "Server listening");
    // dynamic dispatch to NFSv4.0 server implementation
    // TODO add support for multiple NFSv4 minor versions
    let nfs_protocol = NFS40Server::new(root);
    let nfs_server = NFSService::new(nfs_protocol);
    
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let _ = stream.set_nodelay(true);
                info!(%addr, "Client connected");
                let span = span!(Level::TRACE, "client");
                // Reading NFS RPC messages over record marking codec
                let nfs_transport = Framed::new(stream, NFSProtoCodec::new());
                // clone NFS server to move into the pipeline and actor connects with shared state
                let service = nfs_server.clone();
                tokio::spawn(async move { pipeline::Server::new(nfs_transport, service) }.instrument(span).await);
            },
            Err(e) => error!("couldn't get client: {:?}", e),
        }
    }
}

