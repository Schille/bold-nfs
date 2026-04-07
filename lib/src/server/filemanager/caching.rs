use std::collections::{BTreeMap, HashMap};

use bytes::{BufMut, Bytes, BytesMut};
use futures::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use super::{handle::WriteCacheMessage, FilehandleVfs};

#[derive(Debug)]
pub struct WriteCache {
    pub buffer: BytesMut,
    pub write_log: BTreeMap<u64, Bytes>,
    pub changed: bool,
    pub filehandle: FilehandleVfs,
    pub receiver: mpsc::Receiver<WriteCacheMessage>,
}

impl WriteCache {
    pub async fn new(
        receiver: mpsc::Receiver<WriteCacheMessage>,
        filehandle: FilehandleVfs,
    ) -> Self {
        let mut buffer = BytesMut::with_capacity(1024 * 1024);
        let mut write_log = BTreeMap::new();
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
            write_log,
            changed: false,
            filehandle,
            receiver,
        }
    }

    pub async fn handle_message(&mut self, msg: WriteCacheMessage) {
        match msg {
            WriteCacheMessage::Write(req) => {
                self.write_log.insert(req.offset, req.data);
                self.changed = true;
            }
            WriteCacheMessage::Commit(req) => {
                // commit cache
                if let Ok(mut file) = self.filehandle.file.create_file().await {
                    // Using create_file() instead of append_file() because we have the whole buffer.
                    // This might be more efficient depending on VFS.
                    let mut result = BytesMut::with_capacity(req.count as usize);
                    let end = req.offset + req.count as u64;
                    let relevant_entries = self.write_log.range(..end);

                    for (&entry_offset, data) in relevant_entries {
                        let entry_len = data.len() as u64;
                        let entry_end = entry_offset + entry_len;

                        // Check if this specific entry actually overlaps with our query
                        if entry_end <= req.offset {
                            continue; // This entry ends before our query starts
                        }

                        // 2. Calculate which part of this entry we need
                        let overlap_start = req.offset.max(entry_offset);
                        let overlap_end = end.min(entry_end);

                        let src_start = (overlap_start - entry_offset) as usize;
                        let src_end = (overlap_end - entry_offset) as usize;

                        // 3. Copy the sub-slice into our result buffer
                        result.put_slice(&data[src_start..src_end]);
                    }
                    let slice = result.freeze();

                    let _ = file.write_all(&slice).await;
                    let _ = file.flush().await;
                }
            }
        }
    }
}

// WriteCache is run as with the actor pattern
// learn more: https://ryhl.io/blog/actors-with-tokio/ (concept applies to monoio too)
pub async fn run_file_write_cache(mut actor: WriteCache) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}
