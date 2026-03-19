pub mod nfs4_proto;
pub mod nfs4_utils;
pub use onc_rpc;
mod tests;

pub use bytes::{Buf, BufMut, Bytes, BytesMut};
pub use onc_rpc::{CallBody, RpcMessage};
use tokio_util::codec::{Decoder, Encoder};
use tracing::{debug, info, trace};

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
        let mut offset = 0;
        let mut is_last = false;
        let mut fragment_count = 0;

        // First pass: validate full message availability
        while !is_last {
            if src.len() < offset + 4 {
                return Ok(None);
            }

            let header_bytes = [
                src[offset],
                src[offset + 1],
                src[offset + 2],
                src[offset + 3],
            ];
            let fragment_header = u32::from_be_bytes(header_bytes) as usize;
            is_last = (fragment_header & (1 << 31)) > 0;
            let length = fragment_header & ((1 << 31) - 1);

            if length > MAX {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Frame of length {} is too large.", length),
                ));
            }

            if src.len() < offset + 4 + length {
                src.reserve(offset + 4 + length - src.len());
                return Ok(None);
            }

            offset += 4 + length;
            fragment_count += 1;
        }

        // Second pass: we have everything.
        if fragment_count == 1 {
            // Optimization for single-fragment messages: just split and return (including header)
            let message_data = src.split_to(offset).freeze();
            let message = RpcMessage::try_from(message_data)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                .map(Some);
            return message;
        } else {
            // Multi-fragment message: concatenate fragments INCLUDING headers
            let message_data = src.split_to(offset).freeze();
            let message = RpcMessage::try_from(message_data)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                .map(Some);
            return message;
        }
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
        message
            .serialise_into(writer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
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
