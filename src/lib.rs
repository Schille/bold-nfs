pub mod proto;
pub mod server;

pub mod bold {
    use anyhow::{anyhow, Error};
    use base64::prelude::*;
    use tracing::{debug, trace};

    use crate::proto::rpc_proto::RpcReplyMsg;
    pub use crate::proto::rpc_proto::{MsgType, RpcCallMsg};
    use crate::proto::{from_bytes, to_bytes};
    pub use crate::server;
}
