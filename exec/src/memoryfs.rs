use bold::vfs;
use serde::Deserialize;

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

pub fn create_memory_fs(fs_root: Directory) -> vfs::VfsPath {
    fn create_dir(fs: &vfs::VfsPath, dir: &Directory) {
        let dir_path = fs.join(&dir.name).unwrap();
        dir_path.create_dir_all().unwrap();
        for node in &dir.contents {
            match node {
                Node::Dir(dir) => create_dir(&dir_path, dir),
                Node::File(file) => {
                    let file_path = dir_path.join(&file.name).unwrap();
                    file_path
                        .create_file()
                        .unwrap()
                        .write_all(file.contents.as_bytes())
                        .unwrap();
                }
            }
        }
    }

    let root: vfs::VfsPath = vfs::MemoryFS::new().into();
    create_dir(&root, &fs_root);
    root
}
