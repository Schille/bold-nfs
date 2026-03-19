use std::hash::Hash;

use bytes::{Buf, Bytes};

use crate::nfs4_proto::xdr::{
    attrlist4, bitmap4, changeid4, fsid4, nfs_fh4, nfs_ftype4, nfs_lease4, nfsstat4, nfstime4,
    utf8str_cs,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum FileAttrValue {
    SupportedAttrs(bitmap4) = 0,
    Type(nfs_ftype4) = 1,
    FhExpireType(u32) = 2,
    Change(changeid4) = 3,
    Size(u64) = 4,
    LinkSupport(bool) = 5,
    SymlinkSupport(bool) = 6,
    NamedAttr(bool) = 7,
    Fsid(fsid4) = 8,
    UniqueHandles(bool) = 9,
    LeaseTime(nfs_lease4) = 10,
    RdattrError(nfsstat4) = 11,
    Acl(Vec<crate::nfs4_proto::xdr::nfsace4<Bytes>>) = 12,
    AclSupport(u32) = 13,
    Archive(bool) = 14,
    Cansettime(bool) = 15,
    CaseInsensitive(bool) = 16,
    CasePreserving(bool) = 17,
    ChownRestricted(bool) = 18,
    Filehandle(nfs_fh4<Bytes>) = 19,
    Fileid(u64) = 20,
    FilesAvail(u64) = 21,
    FilesFree(u64) = 22,
    FilesTotal(u64) = 23,
    FsLocations(crate::nfs4_proto::xdr::fs_locations4) = 24,
    Hidden(bool) = 25,
    Homogeneous(bool) = 26,
    Maxfilesize(u64) = 27,
    Maxlink(u32) = 28,
    Maxname(u32) = 29,
    Maxread(u64) = 30,
    Maxwrite(u64) = 31,
    Mimetype(utf8str_cs) = 32,
    Mode(u32) = 33,
    NoTrunc(bool) = 34,
    Numlinks(u32) = 35,
    Owner(utf8str_cs) = 36,
    OwnerGroup(utf8str_cs) = 37,
    QuotaAvailHard(u64) = 38,
    QuotaAvailSoft(u64) = 39,
    QuotaUsed(u64) = 40,
    Rawdev(crate::nfs4_proto::xdr::specdata4) = 41,
    SpaceAvail(u64) = 42,
    SpaceFree(u64) = 43,
    SpaceTotal(u64) = 44,
    SpaceUsed(u64) = 45,
    System(bool) = 46,
    TimeAccess(nfstime4) = 47,
    TimeAccessSet(crate::nfs4_proto::xdr::settime4) = 48,
    TimeBackup(nfstime4) = 49,
    TimeCreate(nfstime4) = 50,
    TimeDelta(nfstime4) = 51,
    TimeMetadata(nfstime4) = 52,
    TimeModify(nfstime4) = 53,
    TimeModifySet(crate::nfs4_proto::xdr::settime4) = 54,
    MountedOnFileid(u64) = 55,
}

impl FileAttrValue {
    pub fn attr(&self) -> FileAttr {
        match self {
            FileAttrValue::SupportedAttrs(_) => FileAttr::SupportedAttrs,
            FileAttrValue::Type(_) => FileAttr::Type,
            FileAttrValue::FhExpireType(_) => FileAttr::FhExpireType,
            FileAttrValue::Change(_) => FileAttr::Change,
            FileAttrValue::Size(_) => FileAttr::Size,
            FileAttrValue::LinkSupport(_) => FileAttr::LinkSupport,
            FileAttrValue::SymlinkSupport(_) => FileAttr::SymlinkSupport,
            FileAttrValue::NamedAttr(_) => FileAttr::NamedAttr,
            FileAttrValue::Fsid(_) => FileAttr::Fsid,
            FileAttrValue::UniqueHandles(_) => FileAttr::UniqueHandles,
            FileAttrValue::LeaseTime(_) => FileAttr::LeaseTime,
            FileAttrValue::RdattrError(_) => FileAttr::RdattrError,
            FileAttrValue::Acl(_) => FileAttr::Acl,
            FileAttrValue::AclSupport(_) => FileAttr::AclSupport,
            FileAttrValue::Archive(_) => FileAttr::Archive,
            FileAttrValue::Cansettime(_) => FileAttr::Cansettime,
            FileAttrValue::CaseInsensitive(_) => FileAttr::CaseInsensitive,
            FileAttrValue::CasePreserving(_) => FileAttr::CasePreserving,
            FileAttrValue::ChownRestricted(_) => FileAttr::ChownRestricted,
            FileAttrValue::Filehandle(_) => FileAttr::Filehandle,
            FileAttrValue::Fileid(_) => FileAttr::Fileid,
            FileAttrValue::FilesAvail(_) => FileAttr::FilesAvail,
            FileAttrValue::FilesFree(_) => FileAttr::FilesFree,
            FileAttrValue::FilesTotal(_) => FileAttr::FilesTotal,
            FileAttrValue::FsLocations(_) => FileAttr::FsLocations,
            FileAttrValue::Hidden(_) => FileAttr::Hidden,
            FileAttrValue::Homogeneous(_) => FileAttr::Homogeneous,
            FileAttrValue::Maxfilesize(_) => FileAttr::Maxfilesize,
            FileAttrValue::Maxlink(_) => FileAttr::Maxlink,
            FileAttrValue::Maxname(_) => FileAttr::Maxname,
            FileAttrValue::Maxread(_) => FileAttr::Maxread,
            FileAttrValue::Maxwrite(_) => FileAttr::Maxwrite,
            FileAttrValue::Mimetype(_) => FileAttr::Mimetype,
            FileAttrValue::Mode(_) => FileAttr::Mode,
            FileAttrValue::NoTrunc(_) => FileAttr::NoTrunc,
            FileAttrValue::Numlinks(_) => FileAttr::Numlinks,
            FileAttrValue::Owner(_) => FileAttr::Owner,
            FileAttrValue::OwnerGroup(_) => FileAttr::OwnerGroup,
            FileAttrValue::QuotaAvailHard(_) => FileAttr::QuotaAvailHard,
            FileAttrValue::QuotaAvailSoft(_) => FileAttr::QuotaAvailSoft,
            FileAttrValue::QuotaUsed(_) => FileAttr::QuotaUsed,
            FileAttrValue::Rawdev(_) => FileAttr::Rawdev,
            FileAttrValue::SpaceAvail(_) => FileAttr::SpaceAvail,
            FileAttrValue::SpaceFree(_) => FileAttr::SpaceFree,
            FileAttrValue::SpaceTotal(_) => FileAttr::SpaceTotal,
            FileAttrValue::SpaceUsed(_) => FileAttr::SpaceUsed,
            FileAttrValue::System(_) => FileAttr::System,
            FileAttrValue::TimeAccess(_) => FileAttr::TimeAccess,
            FileAttrValue::TimeAccessSet(_) => FileAttr::TimeAccessSet,
            FileAttrValue::TimeBackup(_) => FileAttr::TimeBackup,
            FileAttrValue::TimeCreate(_) => FileAttr::TimeCreate,
            FileAttrValue::TimeDelta(_) => FileAttr::TimeDelta,
            FileAttrValue::TimeMetadata(_) => FileAttr::TimeMetadata,
            FileAttrValue::TimeModify(_) => FileAttr::TimeModify,
            FileAttrValue::TimeModifySet(_) => FileAttr::TimeModifySet,
            FileAttrValue::MountedOnFileid(_) => FileAttr::MountedOnFileid,
        }
    }
}

impl crate::nfs4_proto::xdr::Serialisable for FileAttrValue {
    fn serialize(&self, dst: &mut bytes::BytesMut) {
        use crate::nfs4_proto::xdr::Serialisable as _;
        match self {
            FileAttrValue::SupportedAttrs(v) => v.serialize(dst),
            FileAttrValue::Type(v) => (v.clone() as u32).serialize(dst),
            FileAttrValue::FhExpireType(v) => v.serialize(dst),
            FileAttrValue::Change(v) => v.serialize(dst),
            FileAttrValue::Size(v) => v.serialize(dst),
            FileAttrValue::LinkSupport(v) => v.serialize(dst),
            FileAttrValue::SymlinkSupport(v) => v.serialize(dst),
            FileAttrValue::NamedAttr(v) => v.serialize(dst),
            FileAttrValue::Fsid(v) => v.serialize(dst),
            FileAttrValue::UniqueHandles(v) => v.serialize(dst),
            FileAttrValue::LeaseTime(v) => v.serialize(dst),
            FileAttrValue::RdattrError(v) => (v.clone() as u32).serialize(dst),
            FileAttrValue::Acl(v) => v.serialize(dst),
            FileAttrValue::AclSupport(v) => v.serialize(dst),
            FileAttrValue::Archive(v) => v.serialize(dst),
            FileAttrValue::Cansettime(v) => v.serialize(dst),
            FileAttrValue::CaseInsensitive(v) => v.serialize(dst),
            FileAttrValue::CasePreserving(v) => v.serialize(dst),
            FileAttrValue::ChownRestricted(v) => v.serialize(dst),
            FileAttrValue::Filehandle(v) => v.serialize(dst),
            FileAttrValue::Fileid(v) => v.serialize(dst),
            FileAttrValue::FilesAvail(v) => v.serialize(dst),
            FileAttrValue::FilesFree(v) => v.serialize(dst),
            FileAttrValue::FilesTotal(v) => v.serialize(dst),
            FileAttrValue::FsLocations(v) => v.serialize(dst),
            FileAttrValue::Hidden(v) => v.serialize(dst),
            FileAttrValue::Homogeneous(v) => v.serialize(dst),
            FileAttrValue::Maxfilesize(v) => v.serialize(dst),
            FileAttrValue::Maxlink(v) => v.serialize(dst),
            FileAttrValue::Maxname(v) => v.serialize(dst),
            FileAttrValue::Maxread(v) => v.serialize(dst),
            FileAttrValue::Maxwrite(v) => v.serialize(dst),
            FileAttrValue::Mimetype(v) => v.serialize(dst),
            FileAttrValue::Mode(v) => v.serialize(dst),
            FileAttrValue::NoTrunc(v) => v.serialize(dst),
            FileAttrValue::Numlinks(v) => v.serialize(dst),
            FileAttrValue::Owner(v) => v.serialize(dst),
            FileAttrValue::OwnerGroup(v) => v.serialize(dst),
            FileAttrValue::QuotaAvailHard(v) => v.serialize(dst),
            FileAttrValue::QuotaAvailSoft(v) => v.serialize(dst),
            FileAttrValue::QuotaUsed(v) => v.serialize(dst),
            FileAttrValue::Rawdev(v) => v.serialize(dst),
            FileAttrValue::SpaceAvail(v) => v.serialize(dst),
            FileAttrValue::SpaceFree(v) => v.serialize(dst),
            FileAttrValue::SpaceTotal(v) => v.serialize(dst),
            FileAttrValue::SpaceUsed(v) => v.serialize(dst),
            FileAttrValue::System(v) => v.serialize(dst),
            FileAttrValue::TimeAccess(v) => v.serialize(dst),
            FileAttrValue::TimeAccessSet(v) => v.serialize(dst),
            FileAttrValue::TimeBackup(v) => v.serialize(dst),
            FileAttrValue::TimeCreate(v) => v.serialize(dst),
            FileAttrValue::TimeDelta(v) => v.serialize(dst),
            FileAttrValue::TimeMetadata(v) => v.serialize(dst),
            FileAttrValue::TimeModify(v) => v.serialize(dst),
            FileAttrValue::TimeModifySet(v) => v.serialize(dst),
            FileAttrValue::MountedOnFileid(v) => v.serialize(dst),
        }
    }
}

impl FileAttr {
    pub fn from_u32(val: u32) -> Option<Self> {
        match val {
            0 => Some(FileAttr::SupportedAttrs),
            1 => Some(FileAttr::Type),
            2 => Some(FileAttr::FhExpireType),
            3 => Some(FileAttr::Change),
            4 => Some(FileAttr::Size),
            5 => Some(FileAttr::LinkSupport),
            6 => Some(FileAttr::SymlinkSupport),
            7 => Some(FileAttr::NamedAttr),
            8 => Some(FileAttr::Fsid),
            9 => Some(FileAttr::UniqueHandles),
            10 => Some(FileAttr::LeaseTime),
            11 => Some(FileAttr::RdattrError),
            12 => Some(FileAttr::Acl),
            13 => Some(FileAttr::AclSupport),
            14 => Some(FileAttr::Archive),
            15 => Some(FileAttr::Cansettime),
            16 => Some(FileAttr::CaseInsensitive),
            17 => Some(FileAttr::CasePreserving),
            18 => Some(FileAttr::ChownRestricted),
            19 => Some(FileAttr::Filehandle),
            20 => Some(FileAttr::Fileid),
            21 => Some(FileAttr::FilesAvail),
            22 => Some(FileAttr::FilesFree),
            23 => Some(FileAttr::FilesTotal),
            24 => Some(FileAttr::FsLocations),
            25 => Some(FileAttr::Hidden),
            26 => Some(FileAttr::Homogeneous),
            27 => Some(FileAttr::Maxfilesize),
            28 => Some(FileAttr::Maxlink),
            29 => Some(FileAttr::Maxname),
            30 => Some(FileAttr::Maxread),
            31 => Some(FileAttr::Maxwrite),
            32 => Some(FileAttr::Mimetype),
            33 => Some(FileAttr::Mode),
            34 => Some(FileAttr::NoTrunc),
            35 => Some(FileAttr::Numlinks),
            36 => Some(FileAttr::Owner),
            37 => Some(FileAttr::OwnerGroup),
            38 => Some(FileAttr::QuotaAvailHard),
            39 => Some(FileAttr::QuotaAvailSoft),
            40 => Some(FileAttr::QuotaUsed),
            41 => Some(FileAttr::Rawdev),
            42 => Some(FileAttr::SpaceAvail),
            43 => Some(FileAttr::SpaceFree),
            44 => Some(FileAttr::SpaceTotal),
            45 => Some(FileAttr::SpaceUsed),
            46 => Some(FileAttr::System),
            47 => Some(FileAttr::TimeAccess),
            48 => Some(FileAttr::TimeAccessSet),
            49 => Some(FileAttr::TimeBackup),
            50 => Some(FileAttr::TimeCreate),
            51 => Some(FileAttr::TimeDelta),
            52 => Some(FileAttr::TimeMetadata),
            53 => Some(FileAttr::TimeModify),
            54 => Some(FileAttr::TimeModifySet),
            55 => Some(FileAttr::MountedOnFileid),
            _ => None,
        }
    }
}

impl Into<Vec<FileAttr>> for bitmap4 {
    fn into(self) -> Vec<FileAttr> {
        let mut attrs = Vec::new();
        for (i, segment) in self.0.iter().enumerate() {
            for bit in 0..32 {
                if (segment.0 >> bit) & 1 == 1 {
                    if let Some(attr) = FileAttr::from_u32((i * 32 + bit) as u32) {
                        attrs.push(attr);
                    }
                }
            }
        }
        attrs
    }
}

impl From<Vec<FileAttr>> for bitmap4 {
    fn from(value: Vec<FileAttr>) -> Self {
        use crate::nfs4_proto::xdr::bitmap_inner;
        let mut bitmap = Vec::new();
        for attr in value {
            let val = attr as u32;
            let i = (val / 32) as usize;
            let bit = val % 32;
            while bitmap.len() <= i {
                bitmap.push(bitmap_inner(0));
            }
            bitmap[i].0 |= 1 << bit;
        }
        bitmap4(bitmap)
    }
}

impl From<Vec<FileAttr>> for attrlist4<Bytes> {
    fn from(_value: Vec<FileAttr>) -> Self {
        // Without values, we can't create an attrlist4.
        // This might be used for GETATTR request bitmap, but that's bitmap4.
        // For SETATTR, we need FileAttrValue.
        attrlist4(Bytes::new())
    }
}

impl From<Vec<FileAttrValue>> for attrlist4<Bytes> {
    fn from(value: Vec<FileAttrValue>) -> Self {
        use crate::nfs4_proto::xdr::Serialisable as _;
        use bytes::BytesMut;

        let mut values = value;
        values.sort_by_key(|v| v.attr() as u32);

        let mut buf = BytesMut::new();
        for val in values {
            val.serialize(&mut buf);
        }
        attrlist4(buf.freeze())
    }
}

impl attrlist4<Bytes> {
    pub fn decode(
        &self,
        mask: &bitmap4,
    ) -> Result<Vec<FileAttrValue>, crate::nfs4_proto::xdr::Error> {
        use crate::nfs4_proto::xdr::DeserialiserExt;
        use std::convert::TryFrom;
        let mut bytes = self.0.clone();
        let attrs: Vec<FileAttr> = mask.clone().into();
        let mut values = Vec::new();

        for attr in attrs {
            match attr {
                FileAttr::SupportedAttrs => values.push(FileAttrValue::SupportedAttrs(
                    bitmap4::try_from(bytes.clone())?,
                )),
                FileAttr::Type => {
                    values.push(FileAttrValue::Type(
                        crate::nfs4_proto::xdr::nfs_ftype4::try_from(bytes.clone())?,
                    ));
                    bytes.advance(4);
                }
                FileAttr::FhExpireType => {
                    values.push(FileAttrValue::FhExpireType(bytes.read_u32()?))
                }
                FileAttr::Change => values.push(FileAttrValue::Change(
                    crate::nfs4_proto::xdr::changeid4(bytes.read_u64()?),
                )),
                FileAttr::Size => values.push(FileAttrValue::Size(bytes.read_u64()?)),
                FileAttr::LinkSupport => {
                    values.push(FileAttrValue::LinkSupport(bytes.read_bool()?))
                }
                FileAttr::SymlinkSupport => {
                    values.push(FileAttrValue::SymlinkSupport(bytes.read_bool()?))
                }
                FileAttr::NamedAttr => values.push(FileAttrValue::NamedAttr(bytes.read_bool()?)),
                FileAttr::Fsid => values.push(FileAttrValue::Fsid(crate::nfs4_proto::xdr::fsid4 {
                    major: bytes.read_u64()?,
                    minor: bytes.read_u64()?,
                })),
                FileAttr::UniqueHandles => {
                    values.push(FileAttrValue::UniqueHandles(bytes.read_bool()?))
                }
                FileAttr::LeaseTime => values.push(FileAttrValue::LeaseTime(
                    crate::nfs4_proto::xdr::nfs_lease4(bytes.read_u32()?),
                )),
                FileAttr::RdattrError => {
                    values.push(FileAttrValue::RdattrError(
                        crate::nfs4_proto::xdr::nfsstat4::try_from(bytes.clone())?,
                    ));
                    bytes.advance(4);
                }
                FileAttr::Acl => values.push(FileAttrValue::Acl(bytes.read_variable_array(None)?)),
                FileAttr::AclSupport => values.push(FileAttrValue::AclSupport(bytes.read_u32()?)),
                FileAttr::Archive => values.push(FileAttrValue::Archive(bytes.read_bool()?)),
                FileAttr::Cansettime => values.push(FileAttrValue::Cansettime(bytes.read_bool()?)),
                FileAttr::CaseInsensitive => {
                    values.push(FileAttrValue::CaseInsensitive(bytes.read_bool()?))
                }
                FileAttr::CasePreserving => {
                    values.push(FileAttrValue::CasePreserving(bytes.read_bool()?))
                }
                FileAttr::ChownRestricted => {
                    values.push(FileAttrValue::ChownRestricted(bytes.read_bool()?))
                }
                FileAttr::Filehandle => values.push(FileAttrValue::Filehandle(
                    crate::nfs4_proto::xdr::nfs_fh4(bytes.read_variable_bytes(None)?),
                )),
                FileAttr::Fileid => values.push(FileAttrValue::Fileid(bytes.read_u64()?)),
                FileAttr::FilesAvail => values.push(FileAttrValue::FilesAvail(bytes.read_u64()?)),
                FileAttr::FilesFree => values.push(FileAttrValue::FilesFree(bytes.read_u64()?)),
                FileAttr::FilesTotal => values.push(FileAttrValue::FilesTotal(bytes.read_u64()?)),
                FileAttr::FsLocations => {
                    // This is complex, but for now we skip or implement enough
                    return Err(crate::nfs4_proto::xdr::Error::Unknown(String::from(
                        "FsLocations decoding not fully implemented",
                    )));
                }
                FileAttr::Hidden => values.push(FileAttrValue::Hidden(bytes.read_bool()?)),
                FileAttr::Homogeneous => {
                    values.push(FileAttrValue::Homogeneous(bytes.read_bool()?))
                }
                FileAttr::Maxfilesize => values.push(FileAttrValue::Maxfilesize(bytes.read_u64()?)),
                FileAttr::Maxlink => values.push(FileAttrValue::Maxlink(bytes.read_u32()?)),
                FileAttr::Maxname => values.push(FileAttrValue::Maxname(bytes.read_u32()?)),
                FileAttr::Maxread => values.push(FileAttrValue::Maxread(bytes.read_u64()?)),
                FileAttr::Maxwrite => values.push(FileAttrValue::Maxwrite(bytes.read_u64()?)),
                FileAttr::Mimetype => values.push(FileAttrValue::Mimetype(
                    crate::nfs4_proto::xdr::utf8str_cs(bytes.read_string(None)?),
                )),
                FileAttr::Mode => values.push(FileAttrValue::Mode(bytes.read_u32()?)),
                FileAttr::NoTrunc => values.push(FileAttrValue::NoTrunc(bytes.read_bool()?)),
                FileAttr::Numlinks => values.push(FileAttrValue::Numlinks(bytes.read_u32()?)),
                FileAttr::Owner => values.push(FileAttrValue::Owner(
                    crate::nfs4_proto::xdr::utf8str_cs(bytes.read_string(None)?),
                )),
                FileAttr::OwnerGroup => values.push(FileAttrValue::OwnerGroup(
                    crate::nfs4_proto::xdr::utf8str_cs(bytes.read_string(None)?),
                )),
                FileAttr::QuotaAvailHard => {
                    values.push(FileAttrValue::QuotaAvailHard(bytes.read_u64()?))
                }
                FileAttr::QuotaAvailSoft => {
                    values.push(FileAttrValue::QuotaAvailSoft(bytes.read_u64()?))
                }
                FileAttr::QuotaUsed => values.push(FileAttrValue::QuotaUsed(bytes.read_u64()?)),
                FileAttr::Rawdev => {
                    let specdata1 = bytes.read_u32()?;
                    let specdata2 = bytes.read_u32()?;
                    values.push(FileAttrValue::Rawdev(crate::nfs4_proto::xdr::specdata4 {
                        specdata1,
                        specdata2,
                    }));
                }
                FileAttr::SpaceAvail => values.push(FileAttrValue::SpaceAvail(bytes.read_u64()?)),
                FileAttr::SpaceFree => values.push(FileAttrValue::SpaceFree(bytes.read_u64()?)),
                FileAttr::SpaceTotal => values.push(FileAttrValue::SpaceTotal(bytes.read_u64()?)),
                FileAttr::SpaceUsed => values.push(FileAttrValue::SpaceUsed(bytes.read_u64()?)),
                FileAttr::System => values.push(FileAttrValue::System(bytes.read_bool()?)),
                FileAttr::TimeAccess => values.push(FileAttrValue::TimeAccess(
                    crate::nfs4_proto::xdr::nfstime4 {
                        seconds: bytes.read_i64()?,
                        nseconds: bytes.read_u32()?,
                    },
                )),
                FileAttr::TimeAccessSet => {
                    // settime4 is an enum
                    return Err(crate::nfs4_proto::xdr::Error::Unknown(String::from(
                        "TimeAccessSet decoding not fully implemented",
                    )));
                }
                FileAttr::TimeBackup => values.push(FileAttrValue::TimeBackup(
                    crate::nfs4_proto::xdr::nfstime4 {
                        seconds: bytes.read_i64()?,
                        nseconds: bytes.read_u32()?,
                    },
                )),
                FileAttr::TimeCreate => values.push(FileAttrValue::TimeCreate(
                    crate::nfs4_proto::xdr::nfstime4 {
                        seconds: bytes.read_i64()?,
                        nseconds: bytes.read_u32()?,
                    },
                )),
                FileAttr::TimeDelta => {
                    values.push(FileAttrValue::TimeDelta(crate::nfs4_proto::xdr::nfstime4 {
                        seconds: bytes.read_i64()?,
                        nseconds: bytes.read_u32()?,
                    }))
                }
                FileAttr::TimeMetadata => values.push(FileAttrValue::TimeMetadata(
                    crate::nfs4_proto::xdr::nfstime4 {
                        seconds: bytes.read_i64()?,
                        nseconds: bytes.read_u32()?,
                    },
                )),
                FileAttr::TimeModify => values.push(FileAttrValue::TimeModify(
                    crate::nfs4_proto::xdr::nfstime4 {
                        seconds: bytes.read_i64()?,
                        nseconds: bytes.read_u32()?,
                    },
                )),
                FileAttr::TimeModifySet => {
                    return Err(crate::nfs4_proto::xdr::Error::Unknown(String::from(
                        "TimeModifySet decoding not fully implemented",
                    )));
                }
                FileAttr::MountedOnFileid => {
                    values.push(FileAttrValue::MountedOnFileid(bytes.read_u64()?))
                }
            }
        }
        Ok(values)
    }
}


impl Hash for nfs_fh4<bytes::Bytes> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Eq for nfs_fh4<bytes::Bytes> {}

#[cfg(test)]
mod nfs4_utils_tests {
    use super::*;
    use crate::nfs4_proto::xdr::{bitmap_inner, nfs_ftype4};
    use bytes::Bytes;

    #[test]
    fn test_bitmap_conversion() {
        let attrs = vec![FileAttr::Type, FileAttr::Size, FileAttr::Mode];
        let bitmap = bitmap4::from(attrs.clone());
        // Type=1, Size=4, Mode=33
        // Segment 0: bits 1, 4
        // Segment 1: bit 1 (33 % 32 = 1)
        assert_eq!(bitmap.0.len(), 2);
        assert_eq!(bitmap.0[0].0, (1 << 1) | (1 << 4));
        assert_eq!(bitmap.0[1].0, (1 << 1));

        let mut back: Vec<FileAttr> = bitmap.into();
        back.sort_by_key(|a| *a as u32);
        assert_eq!(back.len(), 3);
        assert_eq!(back[0], FileAttr::Type);
        assert_eq!(back[1], FileAttr::Size);
        assert_eq!(back[2], FileAttr::Mode);
    }

    #[test]
    fn test_attrlist_serialization_deserialization() {
        let values = vec![
            FileAttrValue::Type(nfs_ftype4::NF4REG),
            FileAttrValue::Size(1024),
            FileAttrValue::Mode(0o644),
        ];

        let bitmap = bitmap4::from(values.iter().map(|v| v.attr()).collect::<Vec<_>>());
        let attrlist = attrlist4::from(values.clone());

        let decoded = attrlist.decode(&bitmap).expect("failed to decode");
        assert_eq!(decoded.len(), 3);
        // They should be in bit order
        assert_eq!(decoded[0], FileAttrValue::Type(nfs_ftype4::NF4REG));
        assert_eq!(decoded[1], FileAttrValue::Size(1024));
        assert_eq!(decoded[2], FileAttrValue::Mode(0o644));
    }
}
