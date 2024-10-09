use std::fmt;

use num_traits::{FromPrimitive, ToPrimitive};
use serde::{
    de::{self, SeqAccess, Visitor},
    ser::{SerializeSeq, SerializeStruct},
    Deserialize, Serialize, Serializer,
};
use tracing::debug;

use crate::nfs4_proto::Compound4args;

use super::{
    nfs4_proto::{Fattr4, FileAttr, FileAttrValue, Getattr4resok, NfsResOp4, NfsStat4},
    rpc_proto::CallBody,
};

pub fn write_argarray<T, S>(v: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<[NfsResOp4]>,
    S: Serializer,
{
    let values = v.as_ref();
    if values.is_empty() {
        serializer.serialize_none()
    } else {
        values.serialize(serializer)
    }
}

pub fn file_attrs_to_bitmap(file_attrs: &Vec<FileAttr>) -> Result<Vec<u32>, anyhow::Error> {
    let mut attrs = Vec::new();
    let mut idxs = file_attrs
        .iter()
        .map(|attr| ToPrimitive::to_u32(attr).unwrap())
        .collect::<Vec<u32>>();

    idxs.reverse();
    let mut segment = 0_u32;
    while let Some(idx) = idxs.pop() {
        // println!("idx: {}", idx);
        // println!("idx.div_ceil(31) {:?}", idx.div_ceil(31));
        if (idx.div_ceil(31) as i16) - 1 > attrs.len() as i16 {
            attrs.push(segment);
            segment = 0_u32;
        }
        segment += 2_u32.pow(idx % 32);
    }
    attrs.push(segment);

    Ok(attrs)
}

pub fn read_attrs<'de, D>(deserializer: D) -> Result<Vec<FileAttr>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let attrs_raw = <Vec<u32> as serde::Deserialize>::deserialize(deserializer).unwrap();

    let mut attrs: Vec<FileAttr> = Vec::new();
    for (idx, segment) in attrs_raw.iter().enumerate() {
        for n in 0..32 {
            let bit = (segment >> n) & 1;
            if bit == 1 {
                let attr: Option<FileAttr> = FromPrimitive::from_u32((idx * 32 + n) as u32);
                if let Some(attr) = attr {
                    attrs.push(attr);
                }
            }
        }
    }
    Ok(attrs)
}

pub fn write_attrs<T, S>(v: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<Vec<FileAttr>>,
    S: Serializer,
{
    let values = v.as_ref();
    let attrs = file_attrs_to_bitmap(values).unwrap();

    let mut seq = serializer.serialize_seq(Some(attrs.len()))?;
    for attr in attrs {
        let _ = seq.serialize_element(&attr);
    }
    seq.end()
}

pub fn read_attr_values<'de, D>(deserializer: D) -> Result<Vec<FileAttrValue>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let _attrs_raw = <u32 as serde::Deserialize>::deserialize(deserializer).unwrap();
    let mut attrs = Vec::new();
    // TODO
    attrs.push(FileAttrValue::Mode(_attrs_raw));
    Ok(attrs)
}

pub fn write_attr_values<T, S>(v: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<[FileAttrValue]>,
    S: Serializer,
{
    let attr_values = v.as_ref();

    let mut buffer: Vec<u8> = Vec::new();

    for val in attr_values {
        // println!("val: {:?}", val);
        match val {
            FileAttrValue::Type(v) => {
                buffer.extend_from_slice(ToPrimitive::to_u32(v).unwrap().to_be_bytes().as_ref());
            }
            FileAttrValue::LeaseTime(v) => {
                buffer.extend_from_slice(v.to_be_bytes().as_ref());
            }
            FileAttrValue::SupportedAttrs(v) => {
                let attrs = file_attrs_to_bitmap(v).unwrap();
                buffer.extend_from_slice((attrs.len() as u32).to_be_bytes().as_ref());
                attrs.iter().for_each(|attr| {
                    buffer.extend_from_slice(attr.to_be_bytes().as_ref());
                });
            }
            FileAttrValue::FhExpireType(v) => {
                buffer.extend_from_slice(v.to_be_bytes().as_ref());
            }
            FileAttrValue::Change(v) => {
                buffer.extend_from_slice(v.to_be_bytes().as_ref());
            }
            FileAttrValue::Size(v) => {
                buffer.extend_from_slice(v.to_be_bytes().as_ref());
            }
            FileAttrValue::LinkSupport(v) => {
                buffer.extend_from_slice((*v as u32).to_be_bytes().as_ref());
            }
            FileAttrValue::SymlinkSupport(v) => {
                buffer.extend_from_slice((*v as u32).to_be_bytes().as_ref());
            }
            FileAttrValue::NamedAttr(v) => {
                buffer.extend_from_slice((*v as u32).to_be_bytes().as_ref());
            }
            FileAttrValue::Fsid(v) => {
                buffer.extend_from_slice(v.major.to_be_bytes().as_ref());
                buffer.extend_from_slice(v.minor.to_be_bytes().as_ref());
            }
            FileAttrValue::UniqueHandles(v) => {
                buffer.extend_from_slice((*v as u32).to_be_bytes().as_ref());
            }
            FileAttrValue::RdattrError(v) => {
                buffer.extend_from_slice(ToPrimitive::to_u32(v).unwrap().to_be_bytes().as_ref());
            }
            FileAttrValue::Fileid(v) => {
                buffer.extend_from_slice(v.to_be_bytes().as_ref());
            }
            FileAttrValue::AclSupport(v) => {
                buffer.extend_from_slice(v.to_be_bytes().as_ref());
            }
            FileAttrValue::Mode(v) => {
                buffer.extend_from_slice(v.to_be_bytes().as_ref());
            }
            FileAttrValue::TimeAccess(v) => {
                buffer.extend_from_slice(v.seconds.to_be_bytes().as_ref());
                buffer.extend_from_slice(v.nseconds.to_be_bytes().as_ref());
            }
            FileAttrValue::TimeModify(v) => {
                buffer.extend_from_slice(v.seconds.to_be_bytes().as_ref());
                buffer.extend_from_slice(v.nseconds.to_be_bytes().as_ref());
            }
            FileAttrValue::TimeMetadata(v) => {
                buffer.extend_from_slice(v.seconds.to_be_bytes().as_ref());
                buffer.extend_from_slice(v.nseconds.to_be_bytes().as_ref());
            }
            FileAttrValue::MountedOnFileid(v) => {
                buffer.extend_from_slice(v.to_be_bytes().as_ref());
            }
            FileAttrValue::Owner(v) => {
                buffer.extend_from_slice((v.len() as u32).to_be_bytes().as_ref());
                buffer.extend_from_slice(v.as_bytes());
            }
            FileAttrValue::OwnerGroup(v) => {
                buffer.extend_from_slice((v.len() as u32).to_be_bytes().as_ref());
                buffer.extend_from_slice(v.as_bytes());
            }
            FileAttrValue::SpaceUsed(v) => {
                buffer.extend_from_slice(v.to_be_bytes().as_ref());
            }
            FileAttrValue::Numlinks(v) => {
                buffer.extend_from_slice(v.to_be_bytes().as_ref());
            }
            _ => {}
        }
    }
    // println!("ser data: {:?}", buffer);
    serializer.serialize_bytes(&buffer)
}

impl Serialize for NfsStat4 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ToPrimitive::to_u32(self).unwrap().serialize(serializer)
    }
}

impl Serialize for Getattr4resok {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.status != NfsStat4::Nfs4Ok {
            debug!("status != NfsStat4::Nfs4Ok: {:?}", self.status);
            let mut seq = serializer.serialize_struct("Getattr4resok", 1)?;
            seq.serialize_field("status", &ToPrimitive::to_u32(&self.status).unwrap())?;
            seq.end()
        } else {
            let mut seq = serializer.serialize_struct("Getattr4resok", 2)?;
            seq.serialize_field("status", &ToPrimitive::to_u32(&self.status).unwrap())?;
            seq.serialize_field("obj_attributes", &self.obj_attributes.as_ref().unwrap())?;
            seq.end()
        }
    }
}

impl<'de> Deserialize<'de> for CallBody {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct CallBodyVisitor;

        impl<'de> Visitor<'de> for CallBodyVisitor {
            type Value = CallBody;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct CallBody")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<CallBody, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let rpcvers = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let prog = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let vers = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let proc = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let cred = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let verf = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                // if proc == 0, then there are no args
                if proc == 0 {
                    // Procedure 0: NULL - No Operation
                    Ok(CallBody {
                        rpcvers,
                        prog,
                        vers,
                        proc,
                        cred,
                        verf,
                        args: None,
                    })
                } else {
                    // Procedure 1: COMPOUND - Compound Operations
                    let args: Compound4args = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                    Ok(CallBody {
                        rpcvers,
                        prog,
                        vers,
                        proc,
                        cred,
                        verf,
                        args: Some(args),
                    })
                }
            }
        }

        const FIELDS: &[&str] = &["rpcvers", "prog", "vers", "proc", "cred", "verf", "args"];
        deserializer.deserialize_struct("CallBody", FIELDS, CallBodyVisitor)
    }
}

// deserialization helper for Fattr4
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FattrRaw {
    attrmask: Vec<u32>,
    #[serde(with = "serde_bytes_ng")]
    attr_vals: Vec<u8>,
}
impl FattrRaw {
    fn to_fileattrs(&self) -> Vec<FileAttr> {
        let mut attrmask: Vec<FileAttr> = Vec::new();
        for (idx, segment) in self.attrmask.iter().enumerate() {
            for n in 0..32 {
                let bit = (segment >> n) & 1;
                if bit == 1 {
                    let attr: Option<FileAttr> = FromPrimitive::from_u32((idx * 32 + n) as u32);
                    if let Some(attr) = attr {
                        attrmask.push(attr);
                    }
                }
            }
        }
        attrmask
    }

    fn to_fileattrvals(&self, fileattrs: &[FileAttr]) -> Vec<FileAttrValue> {
        let mut attr_vals: Vec<FileAttrValue> = Vec::new();
        let mut offset = 0;
        for (idx, attr) in fileattrs.iter().enumerate() {
            match attr {
                FileAttr::Type => {
                    todo!();
                }
                FileAttr::Change => {
                    todo!();
                }
                FileAttr::Size => {
                    todo!();
                }
                FileAttr::TimeAccess => {
                    todo!();
                }
                FileAttr::TimeModify => {
                    todo!();
                }
                FileAttr::TimeMetadata => {
                    todo!();
                }
                FileAttr::MountedOnFileid => {
                    todo!();
                }
                FileAttr::Owner => {
                    todo!();
                }
                FileAttr::OwnerGroup => {
                    todo!();
                }
                FileAttr::SpaceUsed => {
                    todo!();
                }
                FileAttr::Numlinks => {
                    todo!();
                }
                FileAttr::Mode => {
                    let ele =
                        u32::from_be_bytes(self.attr_vals[offset..offset + 4].try_into().unwrap());
                    attr_vals.push(FileAttrValue::Mode(ele));
                    offset += idx + 4;
                }
                _ => todo!(),
            }
        }
        attr_vals
    }
}

impl<'de> Deserialize<'de> for Fattr4 {
    fn deserialize<D>(deserializer: D) -> Result<Fattr4, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let fattr_raw = <FattrRaw as serde::Deserialize>::deserialize(deserializer)?;
        let attrmask = fattr_raw.to_fileattrs();
        let attr_vals = fattr_raw.to_fileattrvals(&attrmask);

        Ok(Fattr4 {
            attrmask,
            attr_vals,
        })
    }
}
