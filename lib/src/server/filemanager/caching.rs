use std::io::{Cursor, Seek, SeekFrom, Write};

use futures::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use super::{handle::WriteCacheMessage, FilehandleVfs};

#[derive(Debug)]
pub struct WriteCache {
    pub filelike: Cursor<Vec<u8>>,
    pub changed: bool,
    pub filehandle: FilehandleVfs,
    pub receiver: mpsc::Receiver<WriteCacheMessage>,
}

impl WriteCache {
    pub async fn new(receiver: mpsc::Receiver<WriteCacheMessage>, filehandle: FilehandleVfs) -> Self {
        let mut filelike = Cursor::new(Vec::new());
        let mut file = filehandle.file.open_file().await.unwrap();
        file.read_to_end(&mut filelike.get_mut()).await.unwrap();
        WriteCache {
            filelike,
            changed: false,
            filehandle,
            receiver,
        }
    }

    pub async fn handle_message(&mut self, msg: WriteCacheMessage) {
        match msg {
            WriteCacheMessage::Write(req) => {
                // write to cache
                self.filelike.seek(SeekFrom::Start(req.offset)).unwrap();
                self.filelike.write_all(req.data.as_ref()).unwrap();
                self.changed = true;
                // update filehandle size (probably not needed here)
                // let new_size = self.filelike.get_ref().len() as u64;
                // self.filehandle.attr_size = new_size;
                // self.filehandle.attr_space_used = new_size;
                // // update change markers
                // self.filehandle.attr_time_modify = Filehandle::attr_time_access();
                // self.filehandle.attr_change =
                //     Filehandle::attr_change(&self.filehandle.file, self.filehandle.version + 1);
                // self.filemanager
                //     .update_filehandle(self.filehandle.clone())
                //     .await;
            }
            WriteCacheMessage::Commit => {
                // commit cache
                if self.changed {
                    let mut file = self.filehandle.file.append_file().await.unwrap();
                    let content = self.filelike.get_ref();
                    let count = file.write(content.as_slice()).await.unwrap() as u32;

                    if count > 0 {
                        file.flush().await.unwrap();
                        // self.filemanager
                        //     .touch_file(self.filehandle.id.clone())
                        //     .await;
                    }
                }
                // self.filemanager
                //     .drop_write_cache_handle(self.filehandle.id.clone())
                //     .await;
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
