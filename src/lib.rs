#![doc = include_str!("../README.md")]

pub mod proto;
pub mod server;

pub mod bold {

    pub use crate::proto::rpc_proto::{MsgType, RpcCallMsg};

    pub use crate::server;
}

#[cfg(test)]
mod test_utils {
    use vfs::{MemoryFS, VfsPath};

    use crate::server::{
        clientmanager::ClientManagerHandle, filemanager::FileManagerHandle, request::NfsRequest,
    };

    fn create_dummyfs() -> VfsPath {
        let root: VfsPath = MemoryFS::new().into();
        root.create_dir_all().unwrap();
        root
    }

    pub async fn create_nfs40_server() -> NfsRequest {
        let root = create_dummyfs();
        let client_mananger_handle = ClientManagerHandle::new();
        let file_mananger_handle = FileManagerHandle::new(root, None);

        NfsRequest::new(
            "127.0.0.1:12345".to_owned(),
            client_mananger_handle,
            file_mananger_handle,
        )
    }
}
