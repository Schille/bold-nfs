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

    use crate::{
        proto::nfs4_proto::{CbClient4, ClientAddr4, NfsClientId4, SetClientId4args},
        server::{
            clientmanager::ClientManagerHandle, filemanager::FileManagerHandle, request::NfsRequest,
        },
    };

    fn create_dummyfs() -> VfsPath {
        let root: VfsPath = MemoryFS::new().into();
        root.create_dir_all().unwrap();
        root
    }

    pub fn create_client(verifier: [u8; 8], id: String) -> SetClientId4args {
        SetClientId4args {
            client: NfsClientId4 { verifier, id },
            callback: CbClient4 {
                cb_program: 0,
                cb_location: ClientAddr4 {
                    rnetid: "tcp".to_string(),
                    raddr: "127.0.0.1.149.18".to_string(),
                },
            },
            callback_ident: 1,
        }
    }

    pub async fn create_nfs40_server(root: Option<VfsPath>) -> NfsRequest {
        let root = if root.is_none() {
            create_dummyfs()
        } else {
            root.unwrap()
        };

        let client_mananger_handle = ClientManagerHandle::new();
        let file_mananger_handle = FileManagerHandle::new(root, None);

        NfsRequest::new(
            "127.0.0.1:12345".to_owned(),
            client_mananger_handle,
            file_mananger_handle,
        )
    }
}
