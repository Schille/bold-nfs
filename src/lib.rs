pub mod server;
pub mod proto;


pub mod nfsv4 {
    use anyhow::{Error, anyhow};
    use base64::prelude::*;
    use tracing::{debug, trace};
    
    use crate::proto::rpc_proto::RpcReplyMsg;
    use crate::proto::{from_bytes, to_bytes};
    pub use crate::server;
    pub use crate::proto::rpc_proto::{MsgType, RpcCallMsg};
    

    // pub fn process_rpc_call<T: server::nfs_server::NFSServer>(call_message: RpcCallMsg, svc: T) -> Result<Box<RpcReplyMsg>, Error> {
    //     // println!("{}", BASE64_STANDARD.encode(&buffer));

    //     // let call_message = from_bytes(buffer);

        
    //     println!("{:?}", call_message);
    //     match call_message.body {
    //         MsgType::Call(ref call_body) => {
    //             match call_body.proc {
    //                 0 => {
    //                     let reply_message = svc.null(call_message);
    //                     trace!("{:?}", reply_message);
    //                     // let resp = to_bytes(reply_message).unwrap();
    //                     return Ok(Box::new(reply_message));
    //                 }
    //                 1 => {
    //                     let reply_message = svc.compound(call_message);
    //                     trace!("{:?}", reply_message);
    //                     // let resp = to_bytes(reply_message).unwrap();
    //                     return Ok(Box::new(reply_message));
    //                 }
    //                 _ => {
    //                     return Err(anyhow!("Invalid procedure"));
    //                 }
    //             }
    //         }
    //         _ => {
    //             return Err(anyhow!("Invalid message type"));
    //         }
    //     }
    
            
        
    

    // }
}

