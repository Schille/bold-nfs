use bold::{AsyncVfsPath, async_vfs::AsyncMemoryFS};
use serde::Deserialize;
use futures::AsyncWriteExt;

#[derive(Deserialize, PartialEq, Debug)]
pub struct Directory {
    name: String,
    contents: Vec<Node>,
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct File {
    name: String,
    contents: String,
}

#[derive(Deserialize, PartialEq, Debug)]
pub enum Node {
    Dir(Directory),
    File(File),
}

pub async fn create_memory_fs(fs_root: Directory) -> AsyncVfsPath {
    async fn create_dir(fs: &AsyncVfsPath, dir: &Directory) {
        let dir_path = fs.join(&dir.name).unwrap();
        dir_path.create_dir_all().await.unwrap();
        for node in &dir.contents {
            match node {
                Node::Dir(dir) => Box::pin(create_dir(&dir_path, dir)).await,
                Node::File(file) => {
                    let file_path = dir_path.join(&file.name).unwrap();
                    file_path.create_file().await.unwrap().write_all(file.contents.as_bytes()).await.unwrap();
                }
            }
        }
    }

    let root = AsyncMemoryFS::new().into();
    create_dir(&root, &fs_root).await;
    root
}
