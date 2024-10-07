use std::{
    hash::{DefaultHasher, Hash, Hasher},
    time::{SystemTime, UNIX_EPOCH},
};

use multi_index_map::MultiIndexMap;
use tracing::debug;
use vfs::VfsPath;

use crate::proto::nfs4_proto::{Fsid4, NfsFtype4, Nfstime4, MODE4_RGRP, MODE4_ROTH, MODE4_RUSR};

use super::locking::LockingState;

pub type FilehandleDb = MultiIndexFilehandleMap;

#[derive(MultiIndexMap, Debug, Clone)]
#[multi_index_derive(Debug, Clone)]
pub struct Filehandle {
    #[multi_index(hashed_unique)]
    pub id: Vec<u8>,
    pub file: VfsPath,
    // path:
    // the full path of the file including filename
    #[multi_index(hashed_unique)]
    pub path: String,
    // type:
    // Designates the type of an object in terms of one of a number of
    // special constants
    pub attr_type: NfsFtype4,
    // change:
    // A value created by the server that the client can use to determine if
    // file data, directory contents, or attributes of the object have been
    // modified.  The server MAY return the object's time_metadata attribute
    // for this attribute's value but only if the file system object cannot
    // be updated more frequently than the resolution of time_metadata.
    pub attr_change: u64,
    // size:
    // The size of the object in bytes.
    pub attr_size: u64,
    // fileid:
    // A number uniquely identifying the file within the file system.
    pub attr_fileid: u64,
    // fsid:
    // Unique file system identifier for the file system holding this
    // object.  The fsid attribute has major and minor components, each of
    // which are of data type uint64_t.
    pub attr_fsid: Fsid4,
    // mode:
    // The NFSv4.0 mode attribute is based on the UNIX mode bits.
    pub attr_mode: u32,
    // owner:
    // The string name of the owner of this object.
    pub attr_owner: String,
    // owner_group:
    // The string name of the group ownership of this object.
    pub attr_owner_group: String,
    // space_used:
    // Number of file system bytes allocated to this object.
    pub attr_space_used: u64,
    // time_access:
    // Represents the time of last access to the object by a READ operation
    // sent to the server.
    pub attr_time_access: Nfstime4,
    // time_metadata:
    // The time of last metadata modification of the object.
    pub attr_time_metadata: Nfstime4,
    // time_modified:
    // The time of last modification to the object.
    pub attr_time_modify: Nfstime4,
    // this filehandle has exclusive open
    pub verifier: Option<[u8; 8]>,
    // attached locks, see LockingState
    // these are not stored in the database, they are retrived on-the-fly
    pub locks: Vec<LockingState>,
    // the version of this filehandle, increased during updates
    pub version: u64,
}

impl Filehandle {
    pub fn new(file: VfsPath, id: Vec<u8>, major: u64, minor: u64, version: u64) -> Self {
        let init_time = Self::attr_time_access();
        let mut path = file.as_str().to_string();
        if path.is_empty() {
            path = "/".to_string();
        }
        let version = version + 1;
        Filehandle {
            id,
            path,
            attr_type: Self::attr_type(&file),
            attr_change: Self::attr_change(&file, version),
            attr_size: Self::attr_size(&file),
            attr_fileid: Self::attr_fileid(&file),
            attr_fsid: Self::attr_fsid(major, minor),
            attr_mode: Self::attr_mode(&file),
            attr_owner: Self::attr_owner(&file),
            attr_owner_group: Self::attr_owner_group(&file),
            attr_space_used: Self::attr_space_used(&file),
            attr_time_access: init_time,
            attr_time_metadata: init_time,
            attr_time_modify: init_time,
            file,
            verifier: None,
            locks: Vec::new(),
            version,
        }
    }

    fn attr_type(file: &VfsPath) -> NfsFtype4 {
        if file.is_dir().unwrap() {
            return NfsFtype4::Nf4dir;
        }
        if file.is_file().unwrap() {
            return NfsFtype4::Nf4reg;
        }
        NfsFtype4::Nf4Undef
    }

    pub fn attr_change(file: &VfsPath, default: u64) -> u64 {
        let v = file.metadata();
        debug!("### attr_change ### {:?}", v);
        if v.is_ok() {
            if let Some(v) = v.unwrap().modified {
                return v.duration_since(UNIX_EPOCH).unwrap().as_secs();
            }
        }
        default
    }

    fn attr_fileid(file: &VfsPath) -> u64 {
        let mut hasher = DefaultHasher::new();
        file.as_str().hash(&mut hasher);

        u64::try_from(hasher.finish()).unwrap()
    }

    fn attr_fsid(major: u64, minor: u64) -> Fsid4 {
        Fsid4 { major, minor }
    }

    fn attr_mode(_file: &VfsPath) -> u32 {
        MODE4_RUSR + MODE4_RGRP + MODE4_ROTH
    }

    fn attr_owner(_file: &VfsPath) -> String {
        "1000".to_string()
    }

    fn attr_owner_group(_file: &VfsPath) -> String {
        "1000".to_string()
    }

    pub fn attr_size(file: &VfsPath) -> u64 {
        file.metadata().unwrap().len
    }

    fn attr_space_used(file: &VfsPath) -> u64 {
        file.metadata().unwrap().len
    }

    pub fn attr_time_access() -> Nfstime4 {
        let since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        Nfstime4 {
            seconds: since_epoch.as_secs() as i64,
            nseconds: since_epoch.subsec_nanos(),
        }
    }
}
