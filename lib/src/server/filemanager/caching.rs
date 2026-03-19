use bytes::BytesMut;
use futures::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use super::{handle::WriteCacheMessage, FilehandleVfs};

#[derive(Debug)]
pub struct WriteCache {
    pub buffer: BytesMut,
    pub changed: bool,
    pub filehandle: FilehandleVfs,
    pub receiver: mpsc::Receiver<WriteCacheMessage>,
}

impl WriteCache {
    pub async fn new(
        receiver: mpsc::Receiver<WriteCacheMessage>,
        filehandle: FilehandleVfs,
    ) -> Self {
        let mut buffer = BytesMut::with_capacity(4096);
        if let Ok(mut file) = filehandle.file.open_file().await {
            // Pre-reserve based on current metadata
            if let Ok(metadata) = filehandle.file.metadata().await {
                let len = metadata.len as usize;
                buffer.resize(len, 0);
                let _ = file.read_exact(&mut buffer).await;
            }
        }

        WriteCache {
            buffer,
            changed: false,
            filehandle,
            receiver,
        }
    }

    pub async fn handle_message(&mut self, msg: WriteCacheMessage) {
        match msg {
            WriteCacheMessage::Write(req) => {
                let offset = req.offset as usize;
                let data_len = req.data.len();
                let end = offset + data_len;

                if end > self.buffer.capacity() {
                    self.buffer.reserve(data_len * 10);
                }

                if end > self.buffer.len() {
                    self.buffer.resize(end, 0);
                }

                // Copy the data into the buffer.
                self.buffer[offset..end].copy_from_slice(req.data.as_ref());
                self.changed = true;
            }
            WriteCacheMessage::Commit => {
                // commit cache
                if self.changed {
                    if let Ok(mut file) = self.filehandle.file.create_file().await {
                        // Using create_file() instead of append_file() because we have the whole buffer.
                        // This might be more efficient depending on VFS.
                        let _ = file.write_all(&self.buffer).await;
                        let _ = file.flush().await;
                    }
                    self.changed = false;
                }
            }
        }
    }
}

// WriteCache is run as with the actor pattern
// learn more: https://ryhl.io/blog/actors-with-tokio/
pub async fn run_file_write_cache(mut actor: WriteCache) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}
