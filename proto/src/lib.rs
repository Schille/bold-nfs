pub mod nfs4_proto;
pub mod nfs4_utils;
pub use onc_rpc;
mod tests;

pub use bytes::{Buf, BufMut, Bytes, BytesMut};
pub use onc_rpc::{CallBody, RpcMessage};
use tokio_util::codec::{Decoder, Encoder};

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
        let mut i = 0;
        let mut is_last = false;
        while !is_last {
            if src.len() < i + 4 {
                // Not enough data to read length marker.
                return Ok(None);
            }

            // Read the frame: https://datatracker.ietf.org/doc/html/rfc1057#section-10
            let header = u32::from_be_bytes([src[i], src[i+1], src[i+2], src[i+3]]);
            is_last = (header & (1 << 31)) > 0;
            let length = (header & 0x7FFF_FFFF) as usize;

            // Check that the length is not too large to avoid a denial of
            // service attack where the server runs out of memory.
            if length > MAX {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Frame of length {} is too large.", length),
                ));
            }

            if src.len() < i + 4 + length {
                // The full fragment has not yet arrived.
                src.reserve(i + 4 + length - src.len());
                return Ok(None);
            }
            i += 4 + length;
        }

        let total_len = i;
        let mem = src.split_to(total_len).freeze();

        let message = RpcMessage::try_from(mem)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            .map(Some);
        message
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
