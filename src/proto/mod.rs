extern crate serde_xdr;
pub mod nfs4_proto;
pub mod rpc_proto;
pub mod utils;

use bytes::{Buf, BytesMut};
use serde_xdr::{from_reader, to_writer, CompatDeserializationError};
use std::io::Cursor;
use tokio_util::codec::{Decoder, Encoder};
use tracing::{debug, event, instrument, trace, Level};

use self::rpc_proto::{RpcCallMsg, RpcReplyMsg};

#[derive(Debug)]
pub struct NFSProtoCodec {}

const MAX: usize = 8 * 1024 * 1024;

impl NFSProtoCodec {
    pub fn new() -> NFSProtoCodec {
        NFSProtoCodec {}
    }
}

impl Decoder for NFSProtoCodec {
    type Item = RpcCallMsg;
    type Error = std::io::Error;

    #[instrument(skip(self), name = "client")]
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut message_data = Vec::new();
        let mut is_last = false;
        while !is_last {
            if src.len() < 4 {
                // Not enough data to read length marker.
                return Ok(None);
            }

            // Read the frame: https://datatracker.ietf.org/doc/html/rfc1057#section-10
            let mut header_bytes = [0u8; 4];
            header_bytes.copy_from_slice(&src[..4]);

            let fragment_header = u32::from_be_bytes(header_bytes) as usize;
            is_last = (fragment_header & (1 << 31)) > 0;
            let length = (fragment_header & ((1 << 31) - 1)) as usize;

            // Check that the length is not too large to avoid a denial of
            // service attack where the server runs out of memory.
            if length > MAX {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Frame of length {} is too large.", length),
                ));
            }

            if src.len() < 4 + length {
                // The full string has not yet arrived.
                src.reserve(4 + length - src.len());
                return Ok(None);
            }
            let fragment = src[4..4 + length].to_vec();
            src.advance(4 + length);

            message_data.extend_from_slice(&fragment[..]);
            trace!(
                length = length,
                is_last = is_last,
                "Finishing Reading fragment"
            );
        }

        RpcCallMsg::from_bytes(message_data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            .map(|msg| Some(msg))
    }
}

impl Encoder<Box<RpcReplyMsg>> for NFSProtoCodec {
    type Error = std::io::Error;

    fn encode(&mut self, message: Box<RpcReplyMsg>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let buffer_message = message
            .to_bytes()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let buffer_header = u32::to_be_bytes(buffer_message.len() as u32 + (1 << 31));
        // debug!("Encoding message : {:?}", buffer_message);
        // Reserve space in the buffer.
        dst.reserve(4 + buffer_message.len());

        // Write the length and string to the buffer.
        dst.extend_from_slice(&buffer_header);
        dst.extend_from_slice(&buffer_message);
        Ok(())
    }
}

pub fn from_bytes(buffer: Vec<u8>) -> Result<RpcCallMsg, anyhow::Error> {
    let mut cursor = Cursor::new(buffer);
    let result: Result<RpcCallMsg, CompatDeserializationError> = from_reader(&mut cursor);
    // todo add proper logging
    match result {
        Ok(msg) => Ok(msg),
        Err(e) => Err(anyhow::anyhow!("Error deserializing message: {:?}", e)),
    }
}

pub fn to_bytes(message: &RpcReplyMsg) -> Result<Vec<u8>, anyhow::Error> {
    let mut bytes = Vec::new();
    let result = to_writer(&mut bytes, message);
    // todo add proper logging
    match result {
        Ok(()) => Ok(bytes),
        Err(e) => Err(anyhow::anyhow!("Error serializing message: {:?}", e)),
    }
}
