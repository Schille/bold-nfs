use std::{
    hash::{DefaultHasher, Hash, Hasher},
    time::{SystemTime, UNIX_EPOCH},
};

use bytes::Bytes;
use multi_index_map::MultiIndexMap;
use tracing::debug;
use vfs::async_vfs::AsyncVfsPath;

use bold_proto::nfs4_proto::xdr::{
    fsid4, nfs_fh4, nfs_ftype4, nfstime4, MODE4_RGRP, MODE4_ROTH, MODE4_RUSR,
};

use super::locking::LockingState;

pub type FilehandleDb = MultiIndexFilehandleVfsMap;

#[derive(MultiIndexMap, Debug, Clone, PartialEq, Eq)]
#[multi_index_derive(Debug, Clone)]
pub struct FilehandleVfs {
    #[multi_index(hashed_unique)]
    pub id: nfs_fh4<Bytes>,
    pub file: AsyncVfsPath,
    // path:
    // the full path of the file including filename
    #[multi_index(hashed_unique)]
    pub path: String,
    // type:
    // Designates the type of an object in terms of one of a number of
    // special constants
    // pub attr_type: nfs_ftype4,
    // change:
    // A value created by the server that the client can use to determine if
    // file data, directory contents, or attributes of the object have been
    // modified.  The server MAY return the object's time_metadata attribute
    // for this attribute's value but only if the file system object cannot
    // be updated more frequently than the resolution of time_metadata.
    // pub attr_change: u64,
    // size:
    // The size of the object in bytes.
    // pub attr_size: u64,
    // fileid:
    // A number uniquely identifying the file within the file system.
    // pub attr_fileid: u64,
    // fsid:
    // Unique file system identifier for the file system holding this
    // object.  The fsid attribute has major and minor components, each of
    // which are of data type uint64_t.
    // pub attr_fsid: fsid4,
    // mode:
    // The NFSv4.0 mode attribute is based on the UNIX mode bits.
    // pub attr_mode: u32,
    // owner:
    // The string name of the owner of this object.
    // pub attr_owner: String,
    // owner_group:
    // The string name of the group ownership of this object.
    // pub attr_owner_group: String,
    // space_used:
    // Number of file system bytes allocated to this object.
    // pub attr_space_used: u64,
    // time_access:
    // Represents the time of last access to the object by a READ operation
    // sent to the server.
    // pub attr_time_access: nfstime4,
    // time_metadata:
    // The time of last metadata modification of the object.
    // pub attr_time_metadata: nfstime4,
    // time_modified:
    // The time of last modification to the object.
    // pub attr_time_modify: nfstime4,
    // this filehandle has exclusive open
    pub verifier: Option<[u8; 8]>,
    // attached locks, see LockingState
    // these are not stored in the database, they are retrived on-the-fly
    pub locks: Vec<LockingState>,
    // the version of this filehandle, increased during updates
    pub version: u64,
}

impl std::hash::Hash for FilehandleVfs {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.path.hash(state);
        self.verifier.hash(state);
        self.version.hash(state);
    }
}

impl FilehandleVfs {
    pub fn new(file: AsyncVfsPath, id: nfs_fh4<Bytes>, version: u64) -> Self {
        // let init_time = Self::attr_time_access();
        let mut path = file.as_str().to_string();
        if path.is_empty() {
            path = "/".to_string();
        }
        let version = version + 1;
        FilehandleVfs {
            id,
            path,
            // attr_type: Self::attr_type(&file),
            // attr_change: Self::attr_change(&file, version),
            // attr_size: Self::attr_size(&file),
            // attr_fileid: Self::attr_fileid(&file),
            // attr_fsid: Self::attr_fsid(major, minor),
            // attr_mode: Self::attr_mode(&file),
            // attr_owner: Self::attr_owner(&file),
            // attr_owner_group: Self::attr_owner_group(&file),
            // attr_space_used: Self::attr_space_used(&file),
            // attr_time_access: init_time,
            // attr_time_metadata: init_time,
            // attr_time_modify: init_time,
            file,
            verifier: None,
            locks: Vec::new(),
            version,
        }
    }

    pub async fn attr_type(&self) -> nfs_ftype4 {
        if self.file.is_dir().await.unwrap() {
            return nfs_ftype4::NF4DIR;
        }
        if self.file.is_file().await.unwrap() {
            return nfs_ftype4::NF4REG;
        }
        nfs_ftype4::NF4SOCK
    }

    pub async fn attr_change(&self, default: u64) -> u64 {
        let v = self.file.metadata().await;
        debug!("### attr_change ### {:?}", v);
        if v.is_ok() {
            if let Some(v) = v.unwrap().modified {
                return v.duration_since(UNIX_EPOCH).unwrap().as_secs();
            }
        }
        default
    }

    pub fn attr_fileid(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.file.as_str().hash(&mut hasher);

        u64::try_from(hasher.finish()).unwrap()
    }

    pub fn attr_fsid(major: u64, minor: u64) -> fsid4 {
        fsid4 { major, minor }
    }

    pub fn attr_mode() -> u32 {
        MODE4_RUSR + MODE4_RGRP + MODE4_ROTH
    }

    pub fn attr_owner(&self) -> String {
        "1000".to_string()
    }

    pub fn attr_owner_group(&self) -> String {
        "1000".to_string()
    }

    pub async fn attr_size(&self) -> u64 {
        self.file.metadata().await.unwrap().len
    }

    pub async fn attr_space_used(&self) -> u64 {
        self.file.metadata().await.unwrap().len
    }

    pub fn attr_time_access(&self) -> nfstime4 {
        // Todo persist
        let since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        nfstime4 {
            seconds: since_epoch.as_secs() as i64,
            nseconds: since_epoch.subsec_nanos(),
        }
    }

    pub fn attr_time_metadata(&self) -> nfstime4 {
        // Todo persist
        let since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        nfstime4 {
            seconds: since_epoch.as_secs() as i64,
            nseconds: since_epoch.subsec_nanos(),
        }
    }

    pub fn attr_time_modify(&self) -> nfstime4 {
        // Todo persist
        let since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        nfstime4 {
            seconds: since_epoch.as_secs() as i64,
            nseconds: since_epoch.subsec_nanos(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfs::async_vfs::AsyncMemoryFS;

    #[tokio::test]
    async fn test_filehandle_vfs_eq() {
        let path: AsyncVfsPath = AsyncMemoryFS::new().into();
        let path1 = path.join("test.txt").unwrap();
        let id1 = nfs_fh4(Bytes::from("1"));
        let id2 = nfs_fh4(Bytes::from("2"));

        let fh1 = FilehandleVfs::new(path1.clone(), id1.clone(), 0);
        let fh2 = FilehandleVfs::new(path1.clone(), id1.clone(), 0);

        assert_eq!(fh1, fh2);
        assert_ne!(fh1, FilehandleVfs::new(path1, id2, 0));
    }
}
