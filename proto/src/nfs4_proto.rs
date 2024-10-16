extern crate serde_bytes;
extern crate serde_xdr;
use super::utils::{read_attrs, write_argarray, write_attr_values, write_attrs};

use num_derive::{FromPrimitive, ToPrimitive};

use serde_derive::{Deserialize, Serialize};

/*
 * This code was derived from RFC 7531.
 */

/*
 *      nfs4_prot.x
 *
 */

/*
 * Basic types for RFC 1832 data type definitions
 */
/*
 * type int          int32_t;
 * type unsigned int u32;
 * type hyper                i64;
 * type unsigned hyper       u64;
 */

/*
 * Sizes
 */
const NFS4_FHSIZE: u32 = 128;
const NFS4_VERIFIER_SIZE: u32 = 8;
const NFS4_OTHER_SIZE: u32 = 12;
const NFS4_OPAQUE_LIMIT: u32 = 1024;

const NFS4_INT64_MAX: i64 = 0x7fffffffffffffff;
const NFS4_UINT64_MAX: u64 = 0xffffffffffffffff;
const NFS4_INT32_MAX: i32 = 0x7fffffff;
const NFS4_UINT32_MAX: u32 = 0xffffffff;

/*
 * File types
 */
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToPrimitive)]
#[repr(u32)]
pub enum NfsFtype4 {
    Nf4Undef = 0,     /* undefined */
    Nf4reg = 1,       /* Regular File */
    Nf4dir = 2,       /* Directory */
    Nf4blk = 3,       /* Special File - block device */
    Nf4chr = 4,       /* Special File - character device */
    Nf4lnk = 5,       /* Symbolic Link */
    Nf4sock = 6,      /* Special File - socket */
    Nf4fifo = 7,      /* Special File - fifo */
    Nf4attrdir = 8,   /* Attribute Directory */
    Nf4namedattr = 9, /* Named Attribute */
}

/*
 * Error status
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, ToPrimitive)]
#[repr(u32)]
pub enum NfsStat4 {
    Nfs4Ok = 0,         /* everything is okay       */
    Nfs4errPerm = 1,    /* caller not privileged    */
    Nfs4errNoent = 2,   /* no such file/directory   */
    Nfs4errIo = 5,      /* hard I/O error           */
    Nfs4errNxio = 6,    /* no such device           */
    Nfs4errAccess = 13, /* access denied            */
    Nfs4errExist = 17,  /* file already exists      */
    Nfs4errXdev = 18,   /* different file systems   */
    /* Unused/reserved        19 */
    Nfs4errNotdir = 20,               /* should be a directory    */
    Nfs4errIsdir = 21,                /* should not be directory  */
    Nfs4errInval = 22,                /* invalid argument         */
    Nfs4errFbig = 27,                 /* file exceeds server max  */
    Nfs4errNospc = 28,                /* no space on file system  */
    Nfs4errRofs = 30,                 /* read-only file system    */
    Nfs4errMlink = 31,                /* too many hard links      */
    Nfs4errNametoolong = 63,          /* name exceeds server max  */
    Nfs4errNotempty = 66,             /* directory not empty      */
    Nfs4errDquot = 69,                /* hard quota limit reached */
    Nfs4errStale = 70,                /* file no longer exists    */
    Nfs4errBadhandle = 10001,         /* Illegal filehandle       */
    Nfs4errBadCookie = 10003,         /* READDIR cookie is stale  */
    Nfs4errNotsupp = 10004,           /* operation not supported  */
    Nfs4errToosmall = 10005,          /* response limit exceeded  */
    Nfs4errServerfault = 10006,       /* undefined server error   */
    Nfs4errBadtype = 10007,           /* type invalid for CREATE  */
    Nfs4errDelay = 10008,             /* file "busy" - retry      */
    Nfs4errSame = 10009,              /* nverify says attrs same  */
    Nfs4errDenied = 10010,            /* lock unavailable         */
    Nfs4errExpired = 10011,           /* lock lease expired       */
    Nfs4errLocked = 10012,            /* I/O failed due to lock   */
    Nfs4errGrace = 10013,             /* in grace period          */
    Nfs4errFhexpired = 10014,         /* filehandle expired       */
    Nfs4errShareDenied = 10015,       /* share reserve denied     */
    Nfs4errWrongsec = 10016,          /* wrong security flavor    */
    Nfs4errClidInuse = 10017,         /* clientid in use          */
    Nfs4errResource = 10018,          /* resource exhaustion      */
    Nfs4errMoved = 10019,             /* file system relocated    */
    Nfs4errNofilehandle = 10020,      /* current FH is not set    */
    Nfs4errMinorVersMismatch = 10021, /* minor vers not supp */
    Nfs4errStaleClientid = 10022,     /* server has rebooted      */
    Nfs4errStaleStateid = 10023,      /* server has rebooted      */
    Nfs4errOldStateid = 10024,        /* state is out of sync     */
    Nfs4errBadStateid = 10025,        /* incorrect stateid        */
    Nfs4errBadSeqid = 10026,          /* request is out of seq.   */
    Nfs4errNotSame = 10027,           /* verify - attrs not same  */
    Nfs4errLockRange = 10028,         /* lock range not supported */
    Nfs4errSymlink = 10029,           /* should be file/directory */
    Nfs4errRestorefh = 10030,         /* no saved filehandle      */
    Nfs4errLeaseMoved = 10031,        /* some file system moved   */
    Nfs4errAttrnotsupp = 10032,       /* recommended attr not sup */
    Nfs4errNoGrace = 10033,           /* reclaim outside of grace */
    Nfs4errReclaimBad = 10034,        /* reclaim error at server  */
    Nfs4errReclaimConflict = 10035,   /* conflict on reclaim    */
    Nfs4errBadxdr = 10036,            /* XDR decode failed        */
    Nfs4errLocksHeld = 10037,         /* file locks held at CLOSE */
    Nfs4errOpenmode = 10038,          /* conflict in OPEN and I/O */
    Nfs4errBadOwner = 10039,          /* Owner translation bad    */
    Nfs4errBadchar = 10040,           /* UTF-8 char not supported */
    Nfs4errBadname = 10041,           /* name not supported       */
    Nfs4errBadRange = 10042,          /* lock range not supported */
    Nfs4errLockNotsupp = 10043,       /* no atomic up/downgrade   */
    Nfs4errOpIllegal = 10044,         /* undefined operation      */
    Nfs4errDeadlock = 10045,          /* file locking deadlock    */
    Nfs4errFileOpen = 10046,          /* open file blocks op.     */
    Nfs4errAdminRevoked = 10047,      /* lock-Owner state revoked */
    Nfs4errCbPathDown = 10048,        /* callback path down       */
}

pub struct FileAttrFlags {}

/*
 * Basic data types
 */
type Attrlist4 = Vec<u8>;
type Bitmap4 = Vec<u32>;
type Changeid4 = u64;
type Clientid4 = u64;
type Count4 = u32;
type Length4 = u64;
type Mode4 = u32;
type NfsCookie4 = u64;
// type NfsFh4 = [u8; NFS4_FHSIZE as usize];
type NfsFh4 = Vec<u8>;
pub type NfsLease4 = u32;
type Offset4 = u64;
type Qop4 = u32;
type SecOid4 = Vec<u64>;
type Seqid4 = u32;
// type opaque  String<>;
type Utf8strCis = String;
type Utf8strCs = String;
type Utf8strMixed = String;
type Component4 = Utf8strCs;
type Linktext4 = Vec<u64>;
type AsciiRequired4 = String;
type Pathname4 = Vec<Component4>;
type NfsLockid4 = u64;
// type Verifier4 = u64;

/*
 * Timeval
 */
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Nfstime4 {
    pub seconds: i64,
    pub nseconds: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimeHow4 {
    SetToServerTime4 = 0,
    SetToClientTime4 = 1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Settime4 {
    time: Nfstime4,
}

/*
 * File attribute definitions
 */

/*
 *  FSID pub structure for major/minor
 */
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fsid4 {
    pub major: u64,
    pub minor: u64,
}

/*
 * File system locations attribute for relocation/migration
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FsLocation4 {
    server: Vec<Utf8strCis>,
    rootpath: Pathname4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FsLocations4 {
    fs_root: Pathname4,
    locations: Vec<FsLocation4>,
}

/*
 * Various Access Control Entry definitions
 */

/*
 * Mask that indicates which Access Control Entries
 * are supported.  Values for the Fattr4_aclsupport attribute.
 */
pub const ACL4_SUPPORT_ALLOW_ACL: u32 = 0x00000001;
pub const ACL4_SUPPORT_DENY_ACL: u32 = 0x00000002;
pub const ACL4_SUPPORT_AUDIT_ACL: u32 = 0x00000004;
pub const ACL4_SUPPORT_ALARM_ACL: u32 = 0x00000008;

type Acetype4 = u32;

/*
 * Acetype4 values; others can be added as needed.
 */
const ACE4_ACCESS_ALLOWED_ACE_TYPE: u32 = 0x00000000;
const ACE4_ACCESS_DENIED_ACE_TYPE: u32 = 0x00000001;
const ACE4_SYSTEM_AUDIT_ACE_TYPE: u32 = 0x00000002;
const ACE4_SYSTEM_ALARM_ACE_TYPE: u32 = 0x00000003;

/*
 * ACE flag
 */
type Aceflag4 = u32;

/*
 * ACE flag values
 */
const ACE4_FILE_INHERIT_ACE: u32 = 0x00000001;
const ACE4_DIRECTORY_INHERIT_ACE: u32 = 0x00000002;
const ACE4_NO_PROPAGATE_INHERIT_ACE: u32 = 0x00000004;
const ACE4_INHERIT_ONLY_ACE: u32 = 0x00000008;
const ACE4_SUCCESSFUL_ACCESS_ACE_FLAG: u32 = 0x00000010;
const ACE4_FAILED_ACCESS_ACE_FLAG: u32 = 0x00000020;
const ACE4_IDENTIFIER_GROUP: u32 = 0x00000040;

/*
 * ACE mask
 */
type Acemask4 = u32;

/*
 * ACE mask values
 */
const ACE4_READ_DATA: u32 = 0x00000001;
const ACE4_LIST_DIRECTORY: u32 = 0x00000001;
const ACE4_WRITE_DATA: u32 = 0x00000002;
const ACE4_ADD_FILE: u32 = 0x00000002;
const ACE4_APPEND_DATA: u32 = 0x00000004;
const ACE4_ADD_SUBDIRECTORY: u32 = 0x00000004;
const ACE4_READ_NAMED_ATTRS: u32 = 0x00000008;
const ACE4_WRITE_NAMED_ATTRS: u32 = 0x00000010;
const ACE4_EXECUTE: u32 = 0x00000020;
const ACE4_DELETE_CHILD: u32 = 0x00000040;
const ACE4_READ_ATTRIBUTES: u32 = 0x00000080;
const ACE4_WRITE_ATTRIBUTES: u32 = 0x00000100;

const ACE4_DELETE: u32 = 0x00010000;
const ACE4_READ_ACL: u32 = 0x00020000;
const ACE4_WRITE_ACL: u32 = 0x00040000;
const ACE4_WRITE_OWNER: u32 = 0x00080000;
const ACE4_SYNCHRONIZE: u32 = 0x00100000;

/*
 * ACE4_GENERIC_READ - defined as a combination of
 *      ACE4_READ_ACL |
 *      ACE4_READ_DATA |
 *      ACE4_READ_ATTRIBUTES |
 *      ACE4_SYNCHRONIZE
 */

const ACE4_GENERIC_READ: u32 = 0x00120081;

/*
 * ACE4_GENERIC_WRITE - defined as a combination of
 *      ACE4_READ_ACL |
 *      ACE4_WRITE_DATA |
 *      ACE4_WRITE_ATTRIBUTES |
 *      ACE4_WRITE_ACL |
 *      ACE4_APPEND_DATA |
 *      ACE4_SYNCHRONIZE
 */
const ACE4_GENERIC_WRITE: u32 = 0x00160106;

/*
 * ACE4_GENERIC_EXECUTE - defined as a combination of
 *      ACE4_READ_ACL
 *      ACE4_READ_ATTRIBUTES
 *      ACE4_EXECUTE
 *      ACE4_SYNCHRONIZE
 */
const ACE4_GENERIC_EXECUTE: u32 = 0x001200A0;

/*
 * Access Control Entry definition
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Nfsace4 {
    pub acetype: Acetype4,
    pub flag: Aceflag4,
    pub access_mask: Acemask4,
    pub who: Utf8strMixed,
}

/*
 * Field definitions for the Fattr4_mode attribute
 */
pub const MODE4_SUID: u32 = 0x800; /* set user id on execution */
pub const MODE4_SGID: u32 = 0x400; /* set group id on execution */
pub const MODE4_SVTX: u32 = 0x200; /* save text even after use */
pub const MODE4_RUSR: u32 = 0x100; /* read permission: Owner */
pub const MODE4_WUSR: u32 = 0x080; /* write permission: Owner */
pub const MODE4_XUSR: u32 = 0x040; /* execute permission: Owner */
pub const MODE4_RGRP: u32 = 0x020; /* read permission: group */
pub const MODE4_WGRP: u32 = 0x010; /* write permission: group */
pub const MODE4_XGRP: u32 = 0x008; /* execute permission: group */
pub const MODE4_ROTH: u32 = 0x004; /* read permission: other */
pub const MODE4_WOTH: u32 = 0x002; /* write permission: other */
pub const MODE4_XOTH: u32 = 0x001; /* execute permission: other */

/*
 * Special data/attribute associated with
 * file types NF4BLK and NF4CHR.
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Specdata4 {
    specdata1: u32, /* major device number */
    specdata2: u32, /* minor device number */
}

/*
 * Values for Fattr4_fh_expire_type
 */
pub const FH4_PERSISTENT: u32 = 0x00000000;
pub const FH4_NOEXPIRE_WITH_OPEN: u32 = 0x00000001;
pub const FH4_VOLATILE_ANY: u32 = 0x00000002;
pub const FH4_VOL_MIGRATION: u32 = 0x00000004;
pub const FH4_VOL_RENAME: u32 = 0x00000008;

type Fattr4SupportedAttrs = Bitmap4;
type Fattr4Type = NfsFtype4;
type Fattr4FhExpireType = u32;
type Fattr4Change = Changeid4;
type Fattr4Size = u64;
type Fattr4LinkSupport = bool;
type Fattr4SymlinkSupport = bool;
type Fattr4NamedAttr = bool;
type Fattr4Fsid = Fsid4;
type Fattr4UniqueHandles = bool;
type Fattr4LeaseTime = NfsLease4;
type Fattr4RdattrError = NfsStat4;

type Fattr4Acl<Nfsace4> = Vec<Nfsace4>;
type Fattr4Aclsupport = u32;
type Fattr4Archive = bool;
type Fattr4Cansettime = bool;
type Fattr4CaseInsensitive = bool;
type Fattr4CasePreserving = bool;
type Fattr4ChownRestricted = bool;
type Fattr4Fileid = u64;
type Fattr4FilesAvail = u64;
type Fattr4Filehandle = NfsFh4;
type Fattr4FilesFree = u64;
type Fattr4FilesTotal = u64;
type Fattr4FsLocations = FsLocations4;
type Fattr4Hidden = bool;
type Fattr4Homogeneous = bool;
type Fattr4Maxfilesize = u64;
type Fattr4Maxlink = u32;
type Fattr4Maxname = u32;
type Fattr4Maxread = u64;
type Fattr4Maxwrite = u64;
type Fattr4Mimetype = AsciiRequired4;
type Fattr4Mode = Mode4;
type Fattr4MountedOnFileid = u64;
type Fattr4NoTrunc = bool;
type Fattr4Numlinks = u32;
type Fattr4Owner = Utf8strMixed;
type Fattr4OwnerGroup = Utf8strMixed;
type Fattr4QuotaAvailHard = u64;
type Fattr4QuotaAvailSoft = u64;
type Fattr4QuotaUsed = u64;
type Fattr4Rawdev = Specdata4;
type Fattr4SpaceAvail = u64;
type Fattr4SpaceFree = u64;
type Fattr4SpaceTotal = u64;
type Fattr4SpaceUsed = u64;
type Fattr4System = bool;
type Fattr4TimeAccess = Nfstime4;
type Fattr4TimeAccessSet = Settime4;
type Fattr4TimeBackup = Nfstime4;
type Fattr4TimeCreate = Nfstime4;
type Fattr4TimeDelta = Nfstime4;
type Fattr4TimeMetadata = Nfstime4;
type Fattr4TimeModify = Nfstime4;
type Fattr4TimeModifySet = Settime4;

/*
 * Mandatory attributes
 */
pub const FATTR4_SUPPORTED_ATTRS: u32 = 0;
pub const FATTR4_TYPE: u32 = 1;
pub const FATTR4_FH_EXPIRE_TYPE: u32 = 2;
pub const FATTR4_CHANGE: u32 = 3;
pub const FATTR4_SIZE: u32 = 4;
pub const FATTR4_LINK_SUPPORT: u32 = 5;
pub const FATTR4_SYMLINK_SUPPORT: u32 = 6;
pub const FATTR4_NAMED_ATTR: u32 = 7;
pub const FATTR4_FSID: u32 = 8;
pub const FATTR4_UNIQUE_HANDLES: u32 = 9;
pub const FATTR4_LEASE_TIME: u32 = 10;
pub const FATTR4_RDATTR_ERROR: u32 = 11;
pub const FATTR4_FILEHANDLE: u32 = 19;

/*
 * Recommended attributes
 */
pub const FATTR4_ACL: u32 = 12;
pub const FATTR4_ACLSUPPORT: u32 = 13;
pub const FATTR4_ARCHIVE: u32 = 14;
pub const FATTR4_CANSETTIME: u32 = 15;
pub const FATTR4_CASE_INSENSITIVE: u32 = 16;
pub const FATTR4_CASE_PRESERVING: u32 = 17;
pub const FATTR4_CHOWN_RESTRICTED: u32 = 18;
pub const FATTR4_FILEID: u32 = 20;
pub const FATTR4_FILES_AVAIL: u32 = 21;
pub const FATTR4_FILES_FREE: u32 = 22;
pub const FATTR4_FILES_TOTAL: u32 = 23;
pub const FATTR4_FS_LOCATIONS: u32 = 24;
pub const FATTR4_HIDDEN: u32 = 25;
pub const FATTR4_HOMOGENEOUS: u32 = 26;
pub const FATTR4_MAXFILESIZE: u32 = 27;
pub const FATTR4_MAXLINK: u32 = 28;
pub const FATTR4_MAXNAME: u32 = 29;
pub const FATTR4_MAXREAD: u32 = 30;
pub const FATTR4_MAXWRITE: u32 = 31;
pub const FATTR4_MIMETYPE: u32 = 32;
pub const FATTR4_MODE: u32 = 33;
pub const FATTR4_NO_TRUNC: u32 = 34;
pub const FATTR4_NUMLINKS: u32 = 35;
pub const FATTR4_OWNER: u32 = 36;
pub const FATTR4_OWNER_GROUP: u32 = 37;
pub const FATTR4_QUOTA_AVAIL_HARD: u32 = 38;
pub const FATTR4_QUOTA_AVAIL_SOFT: u32 = 39;
pub const FATTR4_QUOTA_USED: u32 = 40;
pub const FATTR4_RAWDEV: u32 = 41;
pub const FATTR4_SPACE_AVAIL: u32 = 42;
pub const FATTR4_SPACE_FREE: u32 = 43;
pub const FATTR4_SPACE_TOTAL: u32 = 44;
pub const FATTR4_SPACE_USED: u32 = 45;
pub const FATTR4_SYSTEM: u32 = 46;
pub const FATTR4_TIME_ACCESS: u32 = 47;
pub const FATTR4_TIME_ACCESS_SET: u32 = 48;
pub const FATTR4_TIME_BACKUP: u32 = 49;
pub const FATTR4_TIME_CREATE: u32 = 50;
pub const FATTR4_TIME_DELTA: u32 = 51;
pub const FATTR4_TIME_METADATA: u32 = 52;
pub const FATTR4_TIME_MODIFY: u32 = 53;
pub const FATTR4_TIME_MODIFY_SET: u32 = 54;
pub const FATTR4_MOUNTED_ON_FILEID: u32 = 55;

/*
 * File attribute container
 */
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Fattr4 {
    #[serde(serialize_with = "write_attrs")]
    pub attrmask: Vec<FileAttr>,
    #[serde(serialize_with = "write_attr_values")]
    pub attr_vals: Vec<FileAttrValue>,
}

/*
 * Change info for the client
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChangeInfo4 {
    pub atomic: bool,
    pub before: Changeid4,
    pub after: Changeid4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClientAddr4 {
    /* see
    pub struct rpcb in RFC 1833 */
    pub rnetid: String, /* network id */
    pub raddr: String,  /* universal address */
}

/*
 * Callback program info as provided by the client
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CbClient4 {
    pub cb_program: u32,
    pub cb_location: ClientAddr4,
}

/*
 * Stateid
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stateid4 {
    pub seqid: u32,
    #[serde(with = "serde_xdr::opaque_data::fixed_length")]
    pub other: [u8; NFS4_OTHER_SIZE as usize],
}

/*
 * Client ID
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NfsClientId4 {
    #[serde(with = "serde_xdr::opaque_data::fixed_length")]
    pub verifier: [u8; 8],
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenOwner4 {
    pub clientid: Clientid4,
    #[serde(with = "serde_bytes_ng")]
    pub owner: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LockOwner4 {
    clientid: Clientid4,
    #[serde(with = "serde_bytes_ng")]
    owner: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum NfsLockType4 {
    ReadLt = 1,
    WriteLt = 2,
    ReadwLt = 3,  /* blocking read */
    WritewLt = 4, /* blocking write */
}

pub const ACCESS4_READ: u32 = 0x00000001;
pub const ACCESS4_LOOKUP: u32 = 0x00000002;
pub const ACCESS4_MODIFY: u32 = 0x00000004;
pub const ACCESS4_EXTEND: u32 = 0x00000008;
pub const ACCESS4_DELETE: u32 = 0x00000010;
pub const ACCESS4_EXECUTE: u32 = 0x00000020;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Access4args {
    /* CURRENT_FH: object */
    pub access: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Access4resok {
    pub supported: u32,
    pub access: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Access4res {
    Resok4(Access4resok),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Close4args {
    /* CURRENT_FH: object */
    pub seqid: Seqid4,
    pub open_stateid: Stateid4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Close4res {
    OpenStateid(Stateid4),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Commit4args {
    /* CURRENT_FH: file */
    offset: Offset4,
    count: Count4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Commit4resok {
    #[serde(with = "serde_xdr::opaque_data::fixed_length")]
    pub writeverf: [u8; 8],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum Commit4res {
    Resok4(Commit4resok),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum Createtype4 {
    Linkdata(Linktext4),
    Devdata(Specdata4),
    /* server should return NFS4ERR_BADTYPE */
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Create4args {
    /* CURRENT_FH: directory for creation */
    objtype: Createtype4,
    objname: Component4,
    createattrs: Fattr4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Create4resok {
    cinfo: ChangeInfo4,
    // #[serde(deserialize_with="read_bitmap", serialize_with="write_bitmap")]
    attrset: Bitmap4, /* attributes set */
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum Create4res {
    Resok4(Create4resok),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegPurge4args {
    clientid: Clientid4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegPurge4res {
    status: NfsStat4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegReturn4args {
    /* CURRENT_FH: delegated file */
    deleg_stateid: Stateid4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelegReturn4res {
    saved_fhtatus: NfsStat4,
}

#[derive(Clone, Debug, Eq, PartialEq, FromPrimitive, ToPrimitive, Serialize)]
#[repr(u32)]
pub enum FileAttr {
    SupportedAttrs = 0,
    Type = 1,
    FhExpireType = 2,
    Change = 3,
    Size = 4,
    LinkSupport = 5,
    SymlinkSupport = 6,
    NamedAttr = 7,
    Fsid = 8,
    UniqueHandles = 9,
    LeaseTime = 10,
    RdattrError = 11,
    Acl = 12,
    AclSupport = 13,
    Archive = 14,
    Cansettime = 15,
    CaseInsensitive = 16,
    CasePreserving = 17,
    ChownRestricted = 18,
    Filehandle = 19,
    Fileid = 20,
    FilesAvail = 21,
    FilesFree = 22,
    FilesTotal = 23,
    FsLocations = 24,
    Hidden = 25,
    Homogeneous = 26,
    Maxfilesize = 27,
    Maxlink = 28,
    Maxname = 29,
    Maxread = 30,
    Maxwrite = 31,
    Mimetype = 32,
    Mode = 33,
    NoTrunc = 34,
    Numlinks = 35,
    Owner = 36,
    OwnerGroup = 37,
    QuotaAvailHard = 38,
    QuotaAvailSoft = 39,
    QuotaUsed = 40,
    Rawdev = 41,
    SpaceAvail = 42,
    SpaceFree = 43,
    SpaceTotal = 44,
    SpaceUsed = 45,
    System = 46,
    TimeAccess = 47,
    TimeAccessSet = 48,
    TimeBackup = 49,
    TimeCreate = 50,
    TimeDelta = 51,
    TimeMetadata = 52,
    TimeModify = 53,
    TimeModifySet = 54,
    MountedOnFileid = 55,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum FileAttrValue {
    #[serde(deserialize_with = "read_attrs", serialize_with = "write_attrs")]
    SupportedAttrs(Vec<FileAttr>) = 0,
    Type(NfsFtype4) = 1,
    FhExpireType(u32) = 2,
    Change(Changeid4) = 3,
    Size(u64) = 4,
    LinkSupport(bool) = 5,
    SymlinkSupport(bool) = 6,
    NamedAttr(bool) = 7,
    Fsid(Fsid4) = 8,
    UniqueHandles(bool) = 9,
    LeaseTime(NfsLease4) = 10,
    RdattrError(NfsStat4) = 11,
    Acl = 12,
    AclSupport(u32) = 13,
    Archive = 14,
    Cansettime = 15,
    CaseInsensitive = 16,
    CasePreserving = 17,
    ChownRestricted = 18,
    Filehandle(NfsFh4) = 19,
    Fileid(u64) = 20,
    FilesAvail = 21,
    FilesFree = 22,
    FilesTotal = 23,
    FsLocations = 24,
    Hidden = 25,
    Homogeneous = 26,
    Maxfilesize = 27,
    Maxlink = 28,
    Maxname = 29,
    Maxread = 30,
    Maxwrite = 31,
    Mimetype = 32,
    Mode(u32) = 33,
    NoTrunc = 34,
    Numlinks(u32) = 35,
    Owner(String) = 36,
    OwnerGroup(String) = 37,
    QuotaAvailHard = 38,
    QuotaAvailSoft = 39,
    QuotaUsed = 40,
    Rawdev = 41,
    SpaceAvail = 42,
    SpaceFree = 43,
    SpaceTotal = 44,
    SpaceUsed(u64) = 45,
    System = 46,
    TimeAccess(Nfstime4) = 47,
    TimeAccessSet = 48,
    TimeBackup = 49,
    TimeCreate = 50,
    TimeDelta = 51,
    TimeMetadata(Nfstime4) = 52,
    TimeModify(Nfstime4) = 53,
    TimeModifySet = 54,
    MountedOnFileid(u64) = 55,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Getattr4args {
    /* CURRENT_FH: directory or file */
    #[serde(deserialize_with = "read_attrs", serialize_with = "write_attrs")]
    pub attr_request: Vec<FileAttr>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Getattr4resok {
    pub status: NfsStat4,
    pub obj_attributes: Option<Fattr4>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum Getattr4res {
    Resok4(Getattr4resok) = 0,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GetFh4resok {
    #[serde(with = "serde_bytes")]
    pub object: NfsFh4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum GetFh4res {
    Resok4(GetFh4resok) = 0,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Link4args {
    /* SAVED_FH: source object */
    /* CURRENT_FH: target directory */
    newname: Component4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Link4resok {
    cinfo: ChangeInfo4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum Link4res {
    Resok4(Link4resok),
}

/*
 * For LOCK, transition from open_Owner to new lock_Owner
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenToLockOwner4 {
    open_seqid: Seqid4,
    open_stateid: Stateid4,
    lock_seqid: Seqid4,
    lock_owner: LockOwner4,
}

/*
 * For LOCK, existing lock_Owner continues to request file locks
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExistLockOwner4 {
    lock_stateid: Stateid4,
    lock_seqid: Seqid4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum Locker4 {
    OpenOwner(OpenToLockOwner4),
    LockOwner(ExistLockOwner4),
}

/*
 * LOCK/Lockt/Locku: Record lock management
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lock4args {
    /* CURRENT_FH: file */
    locktype: NfsLockType4,
    reclaim: bool,
    offset: Offset4,
    length: Length4,
    locker: Locker4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lock4denied {
    offset: Offset4,
    length: Length4,
    locktype: NfsLockType4,
    owner: LockOwner4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lock4resok {
    lock_stateid: Stateid4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum Lock4res {
    Resok4(Lock4resok),
    Denied(Lock4denied),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lockt4args {
    /* CURRENT_FH: file */
    locktype: NfsLockType4,
    offset: Offset4,
    length: Length4,
    owner: LockOwner4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum Lockt4res {
    Denied(Lock4denied),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Locku4args {
    /* CURRENT_FH: file */
    locktype: NfsLockType4,
    seqid: Seqid4,
    lock_stateid: Stateid4,
    offset: Offset4,
    length: Length4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum Locku4res {
    LockStateid(Stateid4),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lookup4args {
    /* CURRENT_FH: directory */
    pub objname: Component4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lookup4res {
    /* CURRENT_FH: object */
    pub status: NfsStat4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LookupP4res {
    /* CURRENT_FH: directory */
    status: NfsStat4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Nverify4args {
    /* CURRENT_FH: object */
    obj_attributes: Fattr4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Nverify4res {
    status: NfsStat4,
}

const OPEN4_SHARE_ACCESS_READ: u32 = 0x00000001;
const OPEN4_SHARE_ACCESS_WRITE: u32 = 0x00000002;
const OPEN4_SHARE_ACCESS_BOTH: u32 = 0x00000003;

const OPEN4_SHARE_DENY_NONE: u32 = 0x00000000;
const OPEN4_SHARE_DENY_READ: u32 = 0x00000001;
const OPEN4_SHARE_DENY_WRITE: u32 = 0x00000002;
const OPEN4_SHARE_DENY_BOTH: u32 = 0x00000003;
/*
 * Various definitions for OPEN
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CreateMode4 {
    UNCHECKED4 = 0,
    GUARDED4 = 1,
    EXCLUSIVE4 = 2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum CreateHow4 {
    UNCHECKED4(Fattr4) = 0,
    // GUARDED4
    GUARDED4(Fattr4) = 1,
    // EXCLUSIVE4
    #[serde(with = "serde_xdr::opaque_data::fixed_length")]
    EXCLUSIVE4([u8; 8]) = 2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum OpenType4 {
    Open4Nocreate = 0,
    Open4Create = 1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum OpenFlag4 {
    // Open4Nocreate
    Open4Nocreate = 0,
    // Open4Create
    How(CreateHow4) = 1,
}

/* Next definitions used for OPEN delegation */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum LimitBy4 {
    NfsLimitSize = 1,
    NfsLimitBlocks = 2, /* others as needed */
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NfsModifiedLimit4 {
    num_blocks: u32,
    bytes_per_block: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum NfsSpaceLimit4 {
    /* limit specified as file size */
    Filesize(u64),
    /* limit specified by number of blocks */
    ModBlocks(NfsModifiedLimit4),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum OpenDelegationType4 {
    OpenDelegateNone = 0,
    OpenDelegateRead = 1,
    OpenDelegateWrite = 2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum OpenClaimType4 {
    ClaimNull = 0,
    ClaimPrevious = 1,
    ClaimDelegateCur = 2,
    ClaimDelegatePrev = 3,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenClaimDelegateCur4 {
    delegate_stateid: Stateid4,
    file: Component4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum OpenClaim4 {
    /*
    * No special rights to file.
    * Ordinary OPEN of the specified file.
    */

    /* CURRENT_FH: directory */
    ClaimNull(Component4) = 0,
    /*
    * Right to the file established by an
    * open previous to server reboot.  File
            * identified by filehandle obtained at
    * that time rather than by name.
    */

    /* CURRENT_FH: file being reclaimed */
    ClaimPrevious(OpenDelegationType4) = 1,

    /*
    * Right to file based on a delegation
    * granted by the server.  File is
    * specified by name.

    */
    /* CURRENT_FH: directory */
    ClaimDelegateCur(OpenClaimDelegateCur4) = 2,

    /*
     * Right to file based on a delegation
     * granted to a previous boot instance
     * of the client.  File is specified by name.
     */
    /* CURRENT_FH: directory */
    ClaimDelegatePrev(Component4) = 3,
}

/*
 * OPEN: Open a file, potentially receiving an open delegation
 */
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Open4args {
    pub seqid: Seqid4,
    pub share_access: u32,
    pub share_deny: u32,
    pub owner: OpenOwner4,
    pub openhow: OpenFlag4,
    pub claim: OpenClaim4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenReadDelegation4 {
    /* Stateid for delegation */
    pub stateid: Stateid4,
    /* Pre-recalled flag for
    delegations obtained
    by reclaim (CLAIM_PREVIOUS). */
    pub recall: bool,
    /* Defines users who don't
    need an ACCESS call to
    open for read. */
    pub permissions: Nfsace4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenWriteDelegation4 {
    /* Stateid for delegation */
    stateid: Stateid4,
    /* Pre-recalled flag for
    delegations obtained
    by reclaim
    (CLAIM_PREVIOUS). */
    recall: bool,
    /* Defines condition that
    the client must check to
    determine whether the
    file needs to be flushed
    to the server on close. */
    space_limit: NfsSpaceLimit4,
    /* Defines users who don't
    need an ACCESS call as
    part of a delegated
    open. */
    permissions: Nfsace4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum OpenDelegation4 {
    None = 0,
    Read(OpenReadDelegation4) = 1,
    Write(OpenWriteDelegation4) = 2,
}

/*
 * Result flags
 */

/* Client must confirm open */
pub const OPEN4_RESULT_CONFIRM: u32 = 0x00000002;
/* Type of file locking behavior at the server */
pub const OPEN4_RESULT_LOCKT_YPE_POSIX: u32 = 0x00000004;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Open4resok {
    /* Stateid for open */
    pub stateid: Stateid4,
    /* Directory change info */
    pub cinfo: ChangeInfo4,
    /* Result flags */
    pub rflags: u32,
    /* attribute set for create */
    #[serde(deserialize_with = "read_attrs", serialize_with = "write_attrs")]
    pub attrset: Vec<FileAttr>,
    /* Info on any open
    delegation */
    pub delegation: OpenDelegation4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Open4res {
    /* CURRENT_FH: opened file */
    Resok4(Open4resok),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenAttr4args {
    /* CURRENT_FH: object */
    createdir: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenAttr4res {
    /* CURRENT_FH: named attr directory */
    status: NfsStat4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenConfirm4args {
    /* CURRENT_FH: opened file */
    open_stateid: Stateid4,
    seqid: Seqid4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenConfirm4resok {
    pub open_stateid: Stateid4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum OpenConfirm4res {
    Resok4(OpenConfirm4resok),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenDowngrade4args {
    /* CURRENT_FH: opened file */
    open_stateid: Stateid4,
    seqid: Seqid4,
    share_access: u32,
    share_deny: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpenDowngrade4resok {
    open_stateid: Stateid4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum OpenDowngrade4res {
    Resok4(OpenDowngrade4resok),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PutFh4args {
    #[serde(with = "serde_bytes_ng")]
    pub object: NfsFh4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PutFh4res {
    /* CURRENT_FH: */
    pub status: NfsStat4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PutPupFh4res {
    /* CURRENT_FH: public fh */
    status: NfsStat4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PutRootFh4res {
    /* CURRENT_FH: root fh */
    pub status: NfsStat4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Read4args {
    /* CURRENT_FH: file */
    pub stateid: Stateid4,
    pub offset: Offset4,
    pub count: Count4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Read4resok {
    pub eof: bool,
    #[serde(with = "serde_bytes_ng")]
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Read4res {
    Resok4(Read4resok),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Readdir4args {
    /* CURRENT_FH: directory */
    pub cookie: NfsCookie4,
    #[serde(with = "serde_xdr::opaque_data::fixed_length")]
    pub cookieverf: [u8; 8],
    pub dircount: Count4,
    pub maxcount: Count4,
    #[serde(deserialize_with = "read_attrs")]
    pub attr_request: Vec<FileAttr>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Entry4 {
    // pub len: u32,
    pub cookie: NfsCookie4,
    pub name: Component4,
    pub attrs: Fattr4,
    pub nextentry: Option<Box<Entry4>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirList4 {
    pub entries: Option<Entry4>,
    pub eof: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReadDir4resok {
    #[serde(with = "serde_xdr::opaque_data::fixed_length")]
    pub cookieverf: [u8; 8],
    pub reply: DirList4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum ReadDir4res {
    Resok4(ReadDir4resok),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReadLink4resok {
    link: Linktext4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum ReadLink4res {
    Resok4(ReadLink4resok),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Remove4args {
    /* CURRENT_FH: directory */
    pub target: Component4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Remove4res {
    pub status: NfsStat4,
    pub cinfo: ChangeInfo4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rename4args {
    /* SAVED_FH: source directory */
    oldname: Component4,
    /* CURRENT_FH: target directory */
    newname: Component4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rename4resok {
    source_cinfo: ChangeInfo4,
    target_cinfo: ChangeInfo4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum Rename4res {
    Resok4(Rename4resok),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Renew4args {
    pub clientid: Clientid4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Renew4res {
    pub status: NfsStat4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreFh4res {
    /* CURRENT_FH: value of saved fh */
    status: NfsStat4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SaveFh4res {
    /* SAVED_FH: value of current fh */
    status: NfsStat4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SecInfo4args {
    /* CURRENT_FH: directory */
    name: Component4,
}

/*
 * From RFC 2203
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum RpcGssSvc {
    RpcGssSvcNone = 1,
    RpcGssSvcIntegrity = 2,
    RpcGssSvcPrivacy = 3,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RpcSecGssInfo {
    oid: SecOid4,
    qop: Qop4,
    service: RpcGssSvc,
}

/* RPCSEC_GSS has a value of '6'.  See RFC 2203 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SeCinfo4 {
    FlavorInfo(RpcSecGssInfo),
}

type SecInfo4resok = Vec<SeCinfo4>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum SecInfo4res {
    Resok4(SecInfo4resok),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SetAttr4args {
    /* CURRENT_FH: target object */
    pub stateid: Stateid4,
    pub obj_attributes: Fattr4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SetAttr4res {
    pub status: NfsStat4,
    // #[serde(deserialize_with="read_bitmap", serialize_with="write_bitmap")]
    pub attrsset: Bitmap4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SetClientId4args {
    pub client: NfsClientId4,
    pub callback: CbClient4,
    pub callback_ident: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SetClientId4resok {
    pub clientid: Clientid4,
    #[serde(with = "serde_xdr::opaque_data::fixed_length")]
    pub setclientid_confirm: [u8; 8],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum SetClientId4res {
    Resok4(SetClientId4resok) = 0,
    ClientUsing(ClientAddr4) = 1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SetClientIdConfirm4args {
    pub clientid: Clientid4,
    #[serde(with = "serde_xdr::opaque_data::fixed_length")]
    pub setclientid_confirm: [u8; 8],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SetClientIdConfirm4res {
    pub status: NfsStat4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Verify4args {
    /* CURRENT_FH: object */
    obj_attributes: Fattr4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Verify4res {
    status: NfsStat4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum StableHow4 {
    Unstable4 = 0,
    DataSync4 = 1,
    FileSync4 = 2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Write4args {
    /* CURRENT_FH: file */
    pub stateid: Stateid4,
    pub offset: Offset4,
    pub stable: StableHow4,
    #[serde(with = "serde_bytes_ng")]
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Write4resok {
    pub count: Count4,
    pub committed: StableHow4,
    #[serde(with = "serde_xdr::opaque_data::fixed_length")]
    pub writeverf: [u8; 8],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum Write4res {
    Resok4(Write4resok) = 0,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReleaseLockowner4args {
    lock_owner: LockOwner4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReleaseLockowner4res {
    status: NfsStat4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Illegal4res {
    status: NfsStat4,
}

/*
 * Operation arrays
 */

#[derive(Clone, Debug, Deserialize)]
#[repr(u32)]
pub enum NfsOpNum4 {
    OpAccess = 3,
    OpClose = 4,
    OpCommit = 5,
    OpCreate = 6,
    OpDelegPurge = 7,
    OpDelegReturn = 8,
    OpGetattr = 9,
    OpGetfh = 10,
    OpLink = 11,
    OpLock = 12,
    OpLockt = 13,
    OpLocku = 14,
    OpLookup = 15,
    OpLookupP = 16,
    OpNverify = 17,
    OpOpen = 18,
    OpOpenattr = 19,
    OpOpenConfirm = 20,
    OpOpenDowngrade = 21,
    OpPutfh = 22,
    OpPutpubfh = 23,
    OpPutrootfh = 24,
    OpRead = 25,
    OpReaddir = 26,
    OpReadlink = 27,
    OpRemove = 28,
    OpRename = 29,
    OpRenew = 30,
    OpRestorefh = 31,
    OpSavefh = 32,
    OpSecinfo = 33,
    OpSetattr = 34,
    OpSetclientid = 35,
    OpSetclientidConfirm = 36,
    OpVerify = 37,
    OpWrite = 38,
    OpReleaseLockowner = 39,
    OpIllegal = 10044,
}

#[derive(Clone, Debug, Deserialize)]
pub enum NfsArgOp4 {
    Opaccess(Access4args),
    Opclose(Close4args),
    Opcommit(Commit4args),
    Opcreate(Create4args),
    Opdelegpurge(DelegPurge4args),
    Opdelegreturn(DelegReturn4args),
    Opgetattr(Getattr4args),
    Oplink(Link4args),
    Oplock(Lock4args),
    Oplockt(Lockt4args),
    Oplocku(Locku4args),
    Oplookup(Lookup4args),
    Opnverify(Nverify4args),
    Opopen(Open4args),
    Opopenattr(OpenAttr4args),
    OpopenConfirm(OpenConfirm4args),

    OpopenDowngrade(OpenDowngrade4args),

    Opputfh(PutFh4args),
    Opread(Read4args),
    Opreaddir(Readdir4args),
    Opremove(Remove4args),
    Oprename(Rename4args),
    Oprenew(Renew4args),
    OpseCinfo(SecInfo4args),
    Opsetattr(SetAttr4args),
    Opsetclientid(SetClientId4args),
    OpsetclientidConfirm(SetClientIdConfirm4args),
    Opverify(Verify4args),
    Opwrite(Write4args),
    OpreleaseLockOwner(ReleaseLockowner4args),
    None,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[repr(u32)]
pub enum NfsArgOp {
    OpUndef0 = 0,
    OpUndef1 = 1,
    OpUndef2 = 2,
    OpAccess(Access4args) = 3,
    Opclose(Close4args) = 4,
    Opcommit(Commit4args) = 5,
    Opcreate(Create4args) = 6,
    Opdelegpurge(DelegPurge4args) = 7,
    Opdelegreturn(DelegReturn4args) = 8,
    Opgetattr(Getattr4args) = 9,
    Opgetfh(()) = 10,
    Oplink(Link4args) = 11,
    Oplock(Lock4args) = 12,
    Oplockt(Lockt4args) = 13,
    Oplocku(Locku4args) = 14,
    Oplookup(Lookup4args) = 15,
    Oplookupp(()) = 16,
    Opnverify(Nverify4args) = 17,
    Opopen(Open4args) = 18,
    Opopenattr(OpenAttr4args) = 19,
    OpopenConfirm(OpenConfirm4args) = 20,

    OpopenDowngrade(OpenDowngrade4args) = 21,

    Opputfh(PutFh4args) = 22,
    Opputpubfh(()) = 23,
    Opputrootfh(()) = 24,
    Opread(Read4args) = 25,
    Opreaddir(Readdir4args) = 26,
    Opreadlink(()) = 27,
    Opremove(Remove4args) = 28,
    Oprename(Rename4args) = 29,
    Oprenew(Renew4args) = 30,
    Oprestorefh(()) = 31,
    Opsavefh(()) = 32,

    OpSecinfo(SecInfo4args) = 33,
    Opsetattr(SetAttr4args) = 34,
    Opsetclientid(SetClientId4args) = 35,
    OpsetclientidConfirm(SetClientIdConfirm4args) = 36,
    Opverify(Verify4args) = 37,
    Opwrite(Write4args) = 38,
    OpreleaseLockOwner(ReleaseLockowner4args) = 39,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum NfsResOp4 {
    OpUndef0 = 0,
    OpUndef1 = 1,
    OpUndef2 = 2,
    OpAccess(Access4res) = 3,
    Opclose(Close4res) = 4,
    Opcommit(Commit4res) = 5,
    Opcreate(Create4res) = 6,
    Opdelegpurge(DelegPurge4res) = 7,
    Opdelegreturn(DelegReturn4res) = 8,
    Opgetattr(Getattr4resok) = 9,
    Opgetfh(GetFh4res) = 10,
    Oplink(Link4res) = 11,
    Oplock(Lock4res) = 12,
    Oplockt(Lockt4res) = 13,
    Oplocku(Locku4res) = 14,
    Oplookup(Lookup4res) = 15,
    Oplookupp(()) = 16,
    Opnverify(Nverify4res) = 17,
    Opopen(Open4res) = 18,
    Opopenattr(OpenAttr4res) = 19,
    OpopenConfirm(OpenConfirm4res) = 20,

    OpopenDowngrade(OpenDowngrade4res) = 21,

    Opputfh(PutFh4res) = 22,
    Opputpubfh(()) = 23,
    Opputrootfh(PutRootFh4res) = 24,
    Opread(Read4res) = 25,
    Opreaddir(ReadDir4res) = 26,
    Opreadlink(()) = 27,
    Opremove(Remove4res) = 28,
    Oprename(Rename4res) = 29,
    Oprenew(Renew4res) = 30,
    Oprestorefh(()) = 31,
    Opsavefh(()) = 32,

    OpSecinfo(SecInfo4res) = 33,
    Opsetattr(SetAttr4res) = 34,
    Opsetclientid(SetClientId4res) = 35,
    OpsetclientidConfirm(SetClientIdConfirm4res) = 36,
    Opverify(Verify4res) = 37,
    Opwrite(Write4res) = 38,
    OpreleaseLockOwner(ReleaseLockowner4res) = 39,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Compound4args {
    pub tag: String,
    pub minor_version: u32,
    pub argarray: Vec<NfsArgOp>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Compound4res {
    pub status: NfsStat4,
    pub tag: String,
    #[serde(serialize_with = "write_argarray")]
    pub resarray: Vec<NfsResOp4>,
}

/*
 * Remote file service routines

program NFS4_PROGRAM {
        version NFS_V4 {
                void
                        NFSPROC4_NULL(void) = 0;

                Compound4res
                        NFSPROC4_COMPOUND(Compound4args) = 1;

        } = 4;
} = 100003;
 */
/*
 * NFS4 callback procedure definitions and program
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CbGetattr4args {
    #[serde(with = "serde_bytes_ng")]
    fh: NfsFh4,
    // #[serde(deserialize_with="read_bitmap", serialize_with="write_bitmap")]
    attr_request: Bitmap4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CbGetattr4resok {
    obj_attributes: Fattr4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum CbGetattr4res {
    Resok4(CbGetattr4resok),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CbRecall4args {
    stateid: Stateid4,
    truncate: bool,
    #[serde(with = "serde_bytes_ng")]
    fh: NfsFh4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CbRecall4res {
    status: NfsStat4,
}

/*
 * CBIllegal: Response for illegal operation numbers
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CBIllegal4res {
    status: NfsStat4,
}

/*
 * Various definitions for CB_COMPOUND
 */
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NfsCbOpNum4 {
    OpCbGetattr = 3,
    OpCbRecall = 4,
    OpCbillegal = 10044,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]

pub enum NfsCbArgOp4 {
    Opcbgetattr(CbGetattr4args),
    Opcbrecall(CbRecall4args),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NfsCbResOp4 {
    Opcbgetattr(CbGetattr4res),
    Opcbrecall(CbRecall4res),
    Opcbillegal(CBIllegal4res),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CbCompound4args {
    tag: Utf8strCs,
    minorversion: u32,
    callback_ident: u32,
    argarray: Vec<NfsCbArgOp4>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CbCompound4res {
    status: NfsStat4,
    tag: Utf8strCs,
    resarray: Vec<NfsCbResOp4>,
}
