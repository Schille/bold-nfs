pub mod nfs4_proto;
pub mod nfs4_utils;
pub use onc_rpc;
mod tests;

pub use bytes::{Buf, BufMut, Bytes, BytesMut};
pub use onc_rpc::{CallBody, RpcMessage};
use tokio_util::codec::{Decoder, Encoder};
use tracing::{debug, trace};

#[derive(Debug)]
pub struct XDRProtoCodec {}

const MAX: usize = 8 * 1024 * 1024;

impl Default for XDRProtoCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl XDRProtoCodec {
    pub fn new() -> XDRProtoCodec {
        XDRProtoCodec {}
    }
}

impl Decoder for XDRProtoCodec {
    type Item = RpcMessage<Bytes, Bytes>;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // let mut message_data = Vec::new();
        // let mut is_last = false;
        // while !is_last {
        //     if src.len() < 4 {
        //         // Not enough data to read length marker.
        //         return Ok(None);
        //     }

        //     // Read the frame: https://datatracker.ietf.org/doc/html/rfc1057#section-10
        //     let mut header_bytes = [0u8; 4];
        //     header_bytes.copy_from_slice(&src[..4]);

        //     let fragment_header = u32::from_be_bytes(header_bytes) as usize;
        //     is_last = (fragment_header & (1 << 31)) > 0;
        //     let length = fragment_header & ((1 << 31) - 1);

        //     // Check that the length is not too large to avoid a denial of
        //     // service attack where the server runs out of memory.
        //     if length > MAX {
        //         return Err(std::io::Error::new(
        //             std::io::ErrorKind::InvalidData,
        //             format!("Frame of length {} is too large.", length),
        //         ));
        //     }

        //     if src.len() < 4 + length {
        //         // The full string has not yet arrived.
        //         src.reserve(4 + length - src.len());
        //         return Ok(None);
        //     }
        //     let fragment = src[4..4 + length].to_vec();
        //     src.advance(4 + length);

        //     message_data.extend_from_slice(&fragment[..]);
        //     // TODO remove due to performance reasons
        //     debug!(
        //         length = length,
        //         is_last = is_last,
        //         "Finishing Reading fragment"
        //     );
        // }
        // let mem = Bytes::from_owner(message_data);
        if src.len() < 4 {
            // Not enough data to read length marker.
            return Ok(None);
        }
        let mem = Bytes::from_owner(src.clone());
        let length = mem.len();
        debug!("Received: {:#x}", mem);
        let message = RpcMessage::try_from(mem)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            .map(Some);
        src.advance(length);
        message
        // RpcCallMsg::from_bytes(message_data)
        //     .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        //     .map(Some)
    }
}

impl Encoder<Box<RpcMessage<Bytes, Bytes>>> for XDRProtoCodec {
    type Error = std::io::Error;

    // fn encode(&mut self, message: Box<RpcMessage<Bytes, Bytes>>, dst: &mut BytesMut) -> Result<(), Self::Error> {
    //     let buffer_message = message
    //         .to_bytes()
    //         .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    //     let buffer_header = u32::to_be_bytes(buffer_message.len() as u32 + (1 << 31));
    //     // Reserve space in the buffer.
    //     dst.reserve(4 + buffer_message.len());

    //     // Write the length and string to the buffer.
    //     dst.extend_from_slice(&buffer_header);
    //     dst.extend_from_slice(&buffer_message);
    //     Ok(())
    // }

    fn encode(
        &mut self,
        message: Box<RpcMessage<Bytes, Bytes>>,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        let writer = dst.writer();
        message.serialise_into(writer);
        Ok(())
    }
}

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
