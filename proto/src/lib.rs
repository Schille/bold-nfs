pub mod nfs4_proto;
pub mod nfs4_utils;
pub use onc_rpc;
mod tests;

pub use bytes::{Buf, BufMut, Bytes, BytesMut};
pub use onc_rpc::{CallBody, RpcMessage};
use tracing::{debug, info, trace};

// pub fn from_bytes(buffer: Vec<u8>) -> Result<RpcMessage<Bytes, Bytes>, anyhow::Error> {
//     let mut cursor = Cursor::new(buffer);
//     let result: Result<<Bytes, Bytes>, CompatDeserializationError> = from_reader(&mut cursor);
//     todo add proper logging
//     match result {
//         Ok(msg) => Ok(msg),
//         Err(e) => Err(anyhow::anyhow!("Error deserializing message: {:?}", e)),
//     }
// }

// pub fn to_bytes(message: &RpcReplyMsg) -> Result<Vec<u8>, anyhow::Error> {
//     let mut bytes = Vec::new();
//     let result = to_writer(&mut bytes, message);
//     todo add proper logging
//     match result {
//         Ok(()) => Ok(bytes),
//         Err(e) => Err(anyhow::anyhow!("Error serializing message: {:?}", e)),
//     }
// }
