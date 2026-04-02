pub mod xdr {
    //! An auto-generated set of XDR wire types.
    //!
    //! Do NOT modify the generated file directly.

    #![allow(non_camel_case_types, dead_code, unused_mut, unreachable_patterns)]

    use std::convert::TryFrom;
    use std::fmt::Debug;
    use std::mem::size_of;

    use fastxdr::bytes::{Buf, BufMut, Bytes, BytesMut};
    use fastxdr::thiserror::Error;
    use tracing::debug;

    #[derive(Debug, Error, PartialEq)]
    pub enum Error {
        #[error("invalid message length")]
        InvalidLength,

        #[error("non-utf8 characters in string: {0}")]
        NonUtf8String(#[from] std::string::FromUtf8Error),

        #[error("invalid boolean value")]
        InvalidBoolean,

        #[error("unknown enum variant {0}")]
        UnknownVariant(i32),

        #[error("unknown option variant {0}")]
        UnknownOptionVariant(u32),

        #[error("{0}")]
        Unknown(String),
    }

    pub trait DeserialiserExt {
        type Sliced: WireSize + IntoIterator<Item = u8>;
        type TryFrom;

        fn read_u32(&mut self) -> Result<u32, Error>;
        fn read_u64(&mut self) -> Result<u64, Error>;
        fn read_i32(&mut self) -> Result<i32, Error>;
        fn read_i64(&mut self) -> Result<i64, Error>;
        fn read_f32(&mut self) -> Result<f32, Error>;
        fn read_f64(&mut self) -> Result<f64, Error>;
        fn read_bool(&mut self) -> Result<bool, Error>;
        fn read_bytes(&mut self, n: usize) -> Result<Self::Sliced, Error>;
        fn read_variable_array<T>(&mut self, max: Option<usize>) -> Result<Vec<T>, Error>
        where
            T: TryFrom<Self::TryFrom, Error = Error> + WireSize;

        /// Try to read an opaque XDR array, prefixed by a length u32 and padded
        /// modulo 4.
        fn read_variable_bytes(&mut self, max: Option<usize>) -> Result<Self::Sliced, Error> {
            let n = self.read_u32()? as usize;

            if let Some(limit) = max {
                if n > limit {
                    return Err(Error::InvalidLength);
                }
            }

            self.read_bytes(n)
        }

        /// Reads a variable length UTF8-compatible string from the buffer.
        fn read_string(&mut self, max: Option<usize>) -> Result<String, Error> {
            let b = self
                .read_variable_bytes(max)?
                .into_iter()
                .collect::<Vec<u8>>();
            String::from_utf8(b).map_err(|e| e.into())
        }
    }

    impl DeserialiserExt for Bytes {
        type Sliced = Self;
        type TryFrom = Bytes;

        // Try and read a u32 if self contains enough data.
        fn read_u32(&mut self) -> Result<u32, Error> {
            if self.remaining() < size_of::<u32>() {
                return Err(Error::InvalidLength);
            }
            Ok(self.get_u32())
        }

        fn read_u64(&mut self) -> Result<u64, Error> {
            if self.remaining() < size_of::<u64>() {
                return Err(Error::InvalidLength);
            }
            Ok(self.get_u64())
        }

        fn read_i32(&mut self) -> Result<i32, Error> {
            if self.remaining() < size_of::<i32>() {
                return Err(Error::InvalidLength);
            }
            Ok(self.get_i32())
        }

        fn read_i64(&mut self) -> Result<i64, Error> {
            if self.remaining() < size_of::<i64>() {
                return Err(Error::InvalidLength);
            }
            Ok(self.get_i64())
        }

        fn read_f32(&mut self) -> Result<f32, Error> {
            if self.remaining() < size_of::<f32>() {
                return Err(Error::InvalidLength);
            }
            Ok(self.get_f32())
        }

        fn read_f64(&mut self) -> Result<f64, Error> {
            if self.remaining() < size_of::<f64>() {
                return Err(Error::InvalidLength);
            }
            Ok(self.get_f64())
        }

        fn read_bool(&mut self) -> Result<bool, Error> {
            if self.remaining() < size_of::<i32>() {
                return Err(Error::InvalidLength);
            }
            match self.get_i32() {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(Error::InvalidBoolean),
            }
        }

        /// Try to read an opaque XDR array with a fixed length and padded modulo 4.
        fn read_bytes(&mut self, n: usize) -> Result<Self::Sliced, Error> {
            // Validate the buffer contains enough data
            if self.remaining() < n {
                return Err(Error::InvalidLength);
            }

            let data = self.slice(..n);

            // Advance the buffer cursor, including any padding.
            self.advance(n + pad_length(n));
            Ok(data)
        }

        fn read_variable_array<T>(&mut self, max: Option<usize>) -> Result<Vec<T>, Error>
        where
            T: TryFrom<Self, Error = Error> + WireSize,
        {
            let n = self.read_u32()? as usize;

            if let Some(limit) = max {
                if n > limit {
                    return Err(Error::InvalidLength);
                }
            }

            // Try and decode n instances of T.
            let mut sum = 0;
            let mut out = Vec::with_capacity(n);
            for _ in 0..n {
                let t = T::try_from(self.clone())?;
                if self.remaining() < t.wire_size() {
                    return Err(Error::InvalidLength);
                }
                self.advance(t.wire_size());
                sum += t.wire_size();
                out.push(t);
            }

            self.advance(pad_length(sum));

            Ok(out)
        }
    }

    pub trait WireSize {
        fn wire_size(&self) -> usize;
    }

    pub trait Serialisable {
        /// Serialize all the content in `self`, into `dst`.
        fn serialize(&self, dst: &mut BytesMut);
    }

    macro_rules! wiresize_fixed {
        ($size:literal, $($type:ty),+) => {
            $(
                impl WireSize for $type {
                    fn wire_size(&self) -> usize {
                        $size
                    }
                }
            )+
        };
    }

    wiresize_fixed!(1, u8);
    wiresize_fixed!(4, u32, i32, f32, bool);
    wiresize_fixed!(8, u64, i64, f64);

    impl WireSize for Bytes {
        fn wire_size(&self) -> usize {
            self.len()
        }
    }

    impl<T> WireSize for Vec<T>
    where
        T: WireSize,
    {
        fn wire_size(&self) -> usize {
            // Element count prefix of 4 bytes, plus the individual element lengths
            // (which may vary between elements).
            let x = self.iter().map(|v| v.wire_size()).sum::<usize>();
            4 + x + pad_length(x)
        }
    }

    impl<T> WireSize for [T]
    where
        T: WireSize,
    {
        fn wire_size(&self) -> usize {
            // Individual element lengths (which may vary between elements) without
            // a length byte as [T] is for fixed size arrays.
            let x = self.iter().map(|v| v.wire_size()).sum::<usize>();
            x + pad_length(x)
        }
    }

    impl<T> WireSize for Option<T>
    where
        T: WireSize,
    {
        fn wire_size(&self) -> usize {
            4 + match self {
                Some(inner) => inner.wire_size(),
                None => 0,
            }
        }
    }

    impl<T> WireSize for Box<T>
    where
        T: WireSize,
    {
        fn wire_size(&self) -> usize {
            use std::ops::Deref;
            self.deref().wire_size()
        }
    }

    impl WireSize for String {
        fn wire_size(&self) -> usize {
            4 + self.len() + pad_length(self.len())
        }
    }

    /// Return the amount of padding needed for a value of l bytes in length.
    #[inline]
    fn pad_length(l: usize) -> usize {
        if l % 4 == 0 {
            return 0;
        }
        4 - (l % 4)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use fastxdr::bytes::{BufMut, BytesMut};

        #[derive(Debug, PartialEq)]
        struct TestStruct {
            a: u32,
        }

        impl TryFrom<Bytes> for TestStruct {
            type Error = Error;

            fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
                Ok(Self { a: v.read_u32()? })
            }
        }

        impl WireSize for TestStruct {
            fn wire_size(&self) -> usize {
                self.a.wire_size()
            }
        }

        #[derive(Debug, PartialEq)]
        struct VariableSizedStruct {
            a: Vec<u32>,
        }

        impl TryFrom<Bytes> for VariableSizedStruct {
            type Error = Error;

            fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
                // Stub, always has a len of 2
                let x = v.read_u32()?;
                if x != 2 {
                    panic!("expected len of 2, got {}", x);
                }
                Ok(Self {
                    a: vec![v.read_u32()?, v.read_u32()?],
                })
            }
        }

        impl WireSize for VariableSizedStruct {
            fn wire_size(&self) -> usize {
                self.a.wire_size()
            }
        }

        #[derive(Debug, PartialEq)]
        struct UnalignedStruct {
            a: u8,
        }

        wiresize_fixed!(1, UnalignedStruct);

        impl TryFrom<Bytes> for UnalignedStruct {
            type Error = Error;

            fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
                let s = v.slice(..1);
                v.advance(1);
                Ok(Self { a: s.as_ref()[0] })
            }
        }

        #[test]
        fn test_pad_length() {
            assert_eq!(pad_length(0), 0);
            assert_eq!(pad_length(1), 3);
            assert_eq!(pad_length(2), 2);
            assert_eq!(pad_length(3), 1);
            assert_eq!(pad_length(4), 0);
        }

        #[test]
        fn test_wire_size_basic_types() {
            assert_eq!((42 as u8).wire_size(), 1);
            assert_eq!((42 as u32).wire_size(), 4);
            assert_eq!((42 as i32).wire_size(), 4);
            assert_eq!((42 as u64).wire_size(), 8);
            assert_eq!((42 as i64).wire_size(), 8);
            assert_eq!((42_f32).wire_size(), 4);
            assert_eq!((42_f64).wire_size(), 8);

            // Length prefix of 4 bytes, plus data 5 bytes, plus padding to mod 4
            assert_eq!(String::from("test!").wire_size(), 4 + 5 + 3);

            let mut b = Bytes::new();
            assert_eq!(b.wire_size(), 0);

            let b = BytesMut::new().freeze();
            assert_eq!(b.wire_size(), 0);

            let b = BytesMut::from("test!").freeze();
            assert_eq!(b.wire_size(), 5);

            let b: &[u8] = &[1, 2, 3, 4];
            assert_eq!(b.wire_size(), 4);

            let b: &[u8] = &[1, 2, 3, 4, 5];
            assert_eq!(b.wire_size(), 8); // Padded
        }

        #[test]
        fn test_wire_size_vec() {
            let v1: Vec<u32> = vec![1, 2, 3, 4];
            assert_eq!(v1.wire_size(), 4 * 5);

            let v2: Vec<u64> = vec![1, 2, 3, 4];
            assert_eq!(v2.wire_size(), (8 * 4) + 4);
        }

        #[test]
        fn test_wire_size_array() {
            let v1: [u32; 4] = [1, 2, 3, 4];
            assert_eq!(v1.wire_size(), 4 * 4);

            let v2: [u64; 4] = [1, 2, 3, 4];
            assert_eq!(v2.wire_size(), 8 * 4);
        }

        #[test]
        fn test_variable_array_variable_len_struct() {
            let mut buf = BytesMut::new();
            buf.put_u32(2); // 2 structs

            buf.put_u32(2); // This struct has 2 values
            buf.put_u32(1); // Struct 1
            buf.put_u32(2); // Struct 2

            buf.put_u32(2); // This struct has 2 values
            buf.put_u32(3); // Struct 1
            buf.put_u32(4); // Struct 2

            buf.put_u32(123); // Remaining buffer
            let mut buf = buf.freeze();

            let got = buf
                .read_variable_array::<VariableSizedStruct>(None)
                .unwrap();

            assert_eq!(got.len(), 2);
            assert_eq!(
                got.wire_size(),
                4 + // Variable array length prefix

                4 + // First struct array length prefix
                8 + // First struct data

                4 + // Second struct array length prefix
                8 // Second struct data
            );
            assert_eq!(got[0], VariableSizedStruct { a: vec![1, 2] });

            assert_eq!(buf.len(), 4);
            assert_eq!(buf.as_ref(), &[0, 0, 0, 123]);
        }

        #[test]
        fn test_variable_array_no_max() {
            let mut buf = BytesMut::new();
            buf.put_u32(4); // Len=4
            buf.put_u8(1); // Struct 1
            buf.put_u8(2); // Struct 2
            buf.put_u8(3); // Struct 3
            buf.put_u8(4); // Struct 4
            buf.put_u32(123); // Remaining buffer
            let mut buf = buf.freeze();

            let got = buf.read_variable_array::<UnalignedStruct>(None).unwrap();

            assert_eq!(got.len(), 4);
            assert_eq!(got.wire_size(), 4 + 4); // Inner vecs + vec length

            assert_eq!(got[0], UnalignedStruct { a: 1 });
            assert_eq!(got[0].wire_size(), 1);

            assert_eq!(got[1], UnalignedStruct { a: 2 });
            assert_eq!(got[1].wire_size(), 1);

            assert_eq!(got[2], UnalignedStruct { a: 3 });
            assert_eq!(got[2].wire_size(), 1);

            assert_eq!(got[3], UnalignedStruct { a: 4 });
            assert_eq!(got[3].wire_size(), 1);

            assert_eq!(buf.len(), 4);
            assert_eq!(buf.as_ref(), &[0, 0, 0, 123]);
        }

        #[test]
        fn test_variable_array_no_max_with_padding() {
            let mut buf = BytesMut::new();
            buf.put_u32(2); // Len=4
            buf.put_u8(1); // Struct 1
            buf.put_u8(2); // Struct 2
            buf.put_u8(0); // Padding
            buf.put_u8(0); // Padding
            buf.put_u32(123); // Remaining buffer
            let mut buf = buf.freeze();

            let got = buf.read_variable_array::<UnalignedStruct>(None).unwrap();

            assert_eq!(got.len(), 2);
            assert_eq!(got.wire_size(), 4 + 4);
            assert_eq!(got[0], UnalignedStruct { a: 1 });
            assert_eq!(got[1], UnalignedStruct { a: 2 });

            assert_eq!(buf.len(), 4);
            assert_eq!(buf.as_ref(), &[0, 0, 0, 123]);
        }

        #[test]
        fn test_read_variable_bytes_no_max() {
            let mut buf = BytesMut::new();
            buf.put_u32(8); // Len=8
            buf.put([1, 2, 3, 4, 5, 6, 7, 8].as_ref());
            let mut buf = buf.freeze();

            let got = buf.read_variable_bytes(None).unwrap();

            assert_eq!(got.len(), 8);
            assert_eq!(got.wire_size(), 8);
            assert_eq!(got.as_ref(), &[1, 2, 3, 4, 5, 6, 7, 8]);

            assert_eq!(buf.as_ref(), &[]);
            assert_eq!(buf.remaining(), 0);
        }

        #[test]
        fn test_read_variable_bytes_no_max_with_padding() {
            let mut buf = BytesMut::new();
            buf.put_u32(6); // Len=6 + 2 bytes padding
            buf.put([1, 2, 3, 4, 5, 6, 0, 0].as_ref());
            let mut buf = buf.freeze();

            let got = buf.read_variable_bytes(None).unwrap();

            assert_eq!(got.len(), 6);
            assert_eq!(got.wire_size(), 6);
            assert_eq!(got.as_ref(), &[1, 2, 3, 4, 5, 6]);

            assert_eq!(buf.as_ref(), &[]);
            assert_eq!(buf.remaining(), 0);
        }

        #[test]
        fn test_read_bool() {
            let mut buf = BytesMut::new();
            buf.put_u32(0);
            buf.put_u32(1);
            buf.put_u32(2);
            let mut buf = buf.freeze();

            assert_eq!(buf.read_bool(), Ok(false));
            assert_eq!(buf.read_bool(), Ok(true));
            assert!(buf.read_bool().is_err());
        }
    }
    pub const ACCESS4_DELETE: u32 = 0x00000010;
    pub const ACCESS4_EXECUTE: u32 = 0x00000020;
    pub const ACCESS4_EXTEND: u32 = 0x00000008;
    pub const ACCESS4_LOOKUP: u32 = 0x00000002;
    pub const ACCESS4_MODIFY: u32 = 0x00000004;
    pub const ACCESS4_READ: u32 = 0x00000001;
    pub const ACE4_ACCESS_ALLOWED_ACE_TYPE: u32 = 0x00000000;
    pub const ACE4_ACCESS_DENIED_ACE_TYPE: u32 = 0x00000001;
    pub const ACE4_ADD_FILE: u32 = 0x00000002;
    pub const ACE4_ADD_SUBDIRECTORY: u32 = 0x00000004;
    pub const ACE4_APPEND_DATA: u32 = 0x00000004;
    pub const ACE4_DELETE: u32 = 0x00010000;
    pub const ACE4_DELETE_CHILD: u32 = 0x00000040;
    pub const ACE4_DIRECTORY_INHERIT_ACE: u32 = 0x00000002;
    pub const ACE4_EXECUTE: u32 = 0x00000020;
    pub const ACE4_FAILED_ACCESS_ACE_FLAG: u32 = 0x00000020;
    pub const ACE4_FILE_INHERIT_ACE: u32 = 0x00000001;
    pub const ACE4_GENERIC_EXECUTE: u32 = 0x001200A0;
    pub const ACE4_GENERIC_READ: u32 = 0x00120081;
    pub const ACE4_GENERIC_WRITE: u32 = 0x00160106;
    pub const ACE4_IDENTIFIER_GROUP: u32 = 0x00000040;
    pub const ACE4_INHERIT_ONLY_ACE: u32 = 0x00000008;
    pub const ACE4_LIST_DIRECTORY: u32 = 0x00000001;
    pub const ACE4_NO_PROPAGATE_INHERIT_ACE: u32 = 0x00000004;
    pub const ACE4_READ_ACL: u32 = 0x00020000;
    pub const ACE4_READ_ATTRIBUTES: u32 = 0x00000080;
    pub const ACE4_READ_DATA: u32 = 0x00000001;
    pub const ACE4_READ_NAMED_ATTRS: u32 = 0x00000008;
    pub const ACE4_SUCCESSFUL_ACCESS_ACE_FLAG: u32 = 0x00000010;
    pub const ACE4_SYNCHRONIZE: u32 = 0x00100000;
    pub const ACE4_SYSTEM_ALARM_ACE_TYPE: u32 = 0x00000003;
    pub const ACE4_SYSTEM_AUDIT_ACE_TYPE: u32 = 0x00000002;
    pub const ACE4_WRITE_ACL: u32 = 0x00040000;
    pub const ACE4_WRITE_ATTRIBUTES: u32 = 0x00000100;
    pub const ACE4_WRITE_DATA: u32 = 0x00000002;
    pub const ACE4_WRITE_NAMED_ATTRS: u32 = 0x00000010;
    pub const ACE4_WRITE_OWNER: u32 = 0x00080000;
    pub const ACL4_SUPPORT_ALARM_ACL: u32 = 0x00000008;
    pub const ACL4_SUPPORT_ALLOW_ACL: u32 = 0x00000001;
    pub const ACL4_SUPPORT_AUDIT_ACL: u32 = 0x00000004;
    pub const ACL4_SUPPORT_DENY_ACL: u32 = 0x00000002;
    pub const FATTR4_ACL: u32 = 12;
    pub const FATTR4_ACLSUPPORT: u32 = 13;
    pub const FATTR4_ARCHIVE: u32 = 14;
    pub const FATTR4_CANSETTIME: u32 = 15;
    pub const FATTR4_CASE_INSENSITIVE: u32 = 16;
    pub const FATTR4_CASE_PRESERVING: u32 = 17;
    pub const FATTR4_CHANGE: u32 = 3;
    pub const FATTR4_CHOWN_RESTRICTED: u32 = 18;
    pub const FATTR4_FH_EXPIRE_TYPE: u32 = 2;
    pub const FATTR4_FILEHANDLE: u32 = 19;
    pub const FATTR4_FILEID: u32 = 20;
    pub const FATTR4_FILES_AVAIL: u32 = 21;
    pub const FATTR4_FILES_FREE: u32 = 22;
    pub const FATTR4_FILES_TOTAL: u32 = 23;
    pub const FATTR4_FSID: u32 = 8;
    pub const FATTR4_FS_LOCATIONS: u32 = 24;
    pub const FATTR4_HIDDEN: u32 = 25;
    pub const FATTR4_HOMOGENEOUS: u32 = 26;
    pub const FATTR4_LEASE_TIME: u32 = 10;
    pub const FATTR4_LINK_SUPPORT: u32 = 5;
    pub const FATTR4_MAXFILESIZE: u32 = 27;
    pub const FATTR4_MAXLINK: u32 = 28;
    pub const FATTR4_MAXNAME: u32 = 29;
    pub const FATTR4_MAXREAD: u32 = 30;
    pub const FATTR4_MAXWRITE: u32 = 31;
    pub const FATTR4_MIMETYPE: u32 = 32;
    pub const FATTR4_MODE: u32 = 33;
    pub const FATTR4_MOUNTED_ON_FILEID: u32 = 55;
    pub const FATTR4_NAMED_ATTR: u32 = 7;
    pub const FATTR4_NO_TRUNC: u32 = 34;
    pub const FATTR4_NUMLINKS: u32 = 35;
    pub const FATTR4_OWNER: u32 = 36;
    pub const FATTR4_OWNER_GROUP: u32 = 37;
    pub const FATTR4_QUOTA_AVAIL_HARD: u32 = 38;
    pub const FATTR4_QUOTA_AVAIL_SOFT: u32 = 39;
    pub const FATTR4_QUOTA_USED: u32 = 40;
    pub const FATTR4_RAWDEV: u32 = 41;
    pub const FATTR4_RDATTR_ERROR: u32 = 11;
    pub const FATTR4_SIZE: u32 = 4;
    pub const FATTR4_SPACE_AVAIL: u32 = 42;
    pub const FATTR4_SPACE_FREE: u32 = 43;
    pub const FATTR4_SPACE_TOTAL: u32 = 44;
    pub const FATTR4_SPACE_USED: u32 = 45;
    pub const FATTR4_SUPPORTED_ATTRS: u32 = 0;
    pub const FATTR4_SYMLINK_SUPPORT: u32 = 6;
    pub const FATTR4_SYSTEM: u32 = 46;
    pub const FATTR4_TIME_ACCESS: u32 = 47;
    pub const FATTR4_TIME_ACCESS_SET: u32 = 48;
    pub const FATTR4_TIME_BACKUP: u32 = 49;
    pub const FATTR4_TIME_CREATE: u32 = 50;
    pub const FATTR4_TIME_DELTA: u32 = 51;
    pub const FATTR4_TIME_METADATA: u32 = 52;
    pub const FATTR4_TIME_MODIFY: u32 = 53;
    pub const FATTR4_TIME_MODIFY_SET: u32 = 54;
    pub const FATTR4_TYPE: u32 = 1;
    pub const FATTR4_UNIQUE_HANDLES: u32 = 9;
    pub const FH4_NOEXPIRE_WITH_OPEN: u32 = 0x00000001;
    pub const FH4_PERSISTENT: u32 = 0x00000000;
    pub const FH4_VOLATILE_ANY: u32 = 0x00000002;
    pub const FH4_VOL_MIGRATION: u32 = 0x00000004;
    pub const FH4_VOL_RENAME: u32 = 0x00000008;
    pub const MODE4_RGRP: u32 = 0x020;
    pub const MODE4_ROTH: u32 = 0x004;
    pub const MODE4_RUSR: u32 = 0x100;
    pub const MODE4_SGID: u32 = 0x400;
    pub const MODE4_SUID: u32 = 0x800;
    pub const MODE4_SVTX: u32 = 0x200;
    pub const MODE4_WGRP: u32 = 0x010;
    pub const MODE4_WOTH: u32 = 0x002;
    pub const MODE4_WUSR: u32 = 0x080;
    pub const MODE4_XGRP: u32 = 0x008;
    pub const MODE4_XOTH: u32 = 0x001;
    pub const MODE4_XUSR: u32 = 0x040;
    pub const NFS4_FHSIZE: u32 = 128;
    pub const NFS4_INT32_MAX: u32 = 0x7fffffff;
    pub const NFS4_INT64_MAX: u64 = 0x7fffffffffffffff;
    pub const NFS4_OPAQUE_LIMIT: u32 = 1024;
    pub const NFS4_OTHER_SIZE: u32 = 12;
    pub const NFS4_UINT32_MAX: u32 = 0xffffffff;
    pub const NFS4_UINT64_MAX: u64 = 0xffffffffffffffff;
    pub const NFS4_VERIFIER_SIZE: u32 = 8;
    pub const OPEN4_RESULT_CONFIRM: u32 = 0x00000002;
    pub const OPEN4_RESULT_LOCKTYPE_POSIX: u32 = 0x00000004;
    pub const OPEN4_SHARE_ACCESS_BOTH: u32 = 0x00000003;
    pub const OPEN4_SHARE_ACCESS_READ: u32 = 0x00000001;
    pub const OPEN4_SHARE_ACCESS_WRITE: u32 = 0x00000002;
    pub const OPEN4_SHARE_DENY_BOTH: u32 = 0x00000003;
    pub const OPEN4_SHARE_DENY_NONE: u32 = 0x00000000;
    pub const OPEN4_SHARE_DENY_READ: u32 = 0x00000001;
    pub const OPEN4_SHARE_DENY_WRITE: u32 = 0x00000002;
    #[derive(Debug, PartialEq, Clone)]
    pub struct ACCESS4args {
        pub access: u32,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum ACCESS4res {
        NFS4_OK(ACCESS4resok),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct ACCESS4resok {
        pub supported: u32,
        pub access: u32,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct CB_COMPOUND4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub tag: utf8str_cs,
        pub minorversion: u32,
        pub callback_ident: u32,
        pub argarray: Vec<nfs_cb_argop4<T>>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct CB_COMPOUND4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub status: nfsstat4,
        pub tag: utf8str_cs,
        pub resarray: Vec<nfs_cb_resop4<T>>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct CB_GETATTR4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub fh: nfs_fh4<T>,
        pub attr_request: bitmap4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum CB_GETATTR4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(CB_GETATTR4resok<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct CB_GETATTR4resok<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub obj_attributes: fattr4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct CB_ILLEGAL4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct CB_RECALL4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub stateid: stateid4<T>,
        pub truncate: bool,
        pub fh: nfs_fh4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct CB_RECALL4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct CLOSE4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub seqid: seqid4,
        pub open_stateid: stateid4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum CLOSE4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(stateid4<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct COMMIT4args {
        pub offset: offset4,
        pub count: count4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum COMMIT4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(COMMIT4resok<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct COMMIT4resok<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub writeverf: verifier4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct COMPOUND4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub tag: utf8str_cs,
        pub minorversion: u32,
        pub argarray: Vec<nfs_argop4<T>>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct COMPOUND4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub status: nfsstat4,
        pub tag: utf8str_cs,
        pub resarray: Vec<nfs_resop4<T>>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct CREATE4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub objtype: createtype4<T>,
        pub objname: component4,
        pub createattrs: fattr4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum CREATE4res {
        NFS4_OK(CREATE4resok),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct CREATE4resok {
        pub cinfo: change_info4,
        pub attrset: bitmap4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct DELEGPURGE4args {
        pub clientid: clientid4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct DELEGPURGE4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct DELEGRETURN4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub deleg_stateid: stateid4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct DELEGRETURN4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct GETATTR4args {
        pub attr_request: bitmap4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum GETATTR4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(GETATTR4resok<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct GETATTR4resok<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub obj_attributes: fattr4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum GETFH4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(GETFH4resok<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct GETFH4resok<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub object: nfs_fh4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct ILLEGAL4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct LINK4args {
        pub newname: component4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum LINK4res {
        NFS4_OK(LINK4resok),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct LINK4resok {
        pub cinfo: change_info4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct LOCK4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub locktype: nfs_lock_type4,
        pub reclaim: bool,
        pub offset: offset4,
        pub length: length4,
        pub locker: locker4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct LOCK4denied<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub offset: offset4,
        pub length: length4,
        pub locktype: nfs_lock_type4,
        pub owner: lock_owner4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum LOCK4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(LOCK4resok<T>),
        NFS4ERR_DENIED(LOCK4denied<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct LOCK4resok<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub lock_stateid: stateid4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct LOCKT4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub locktype: nfs_lock_type4,
        pub offset: offset4,
        pub length: length4,
        pub owner: lock_owner4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum LOCKT4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4ERR_DENIED(LOCK4denied<T>),
        NFS4_OK,
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct LOCKU4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub locktype: nfs_lock_type4,
        pub seqid: seqid4,
        pub lock_stateid: stateid4<T>,
        pub offset: offset4,
        pub length: length4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum LOCKU4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(stateid4<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct LOOKUP4args {
        pub objname: component4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct LOOKUP4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct LOOKUPP4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct NVERIFY4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub obj_attributes: fattr4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct NVERIFY4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct OPEN4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub seqid: seqid4,
        pub share_access: u32,
        pub share_deny: u32,
        pub owner: open_owner4<T>,
        pub openhow: openflag4<T>,
        pub claim: open_claim4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum OPEN4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(OPEN4resok<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct OPEN4resok<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub stateid: stateid4<T>,
        pub cinfo: change_info4,
        pub rflags: u32,
        pub attrset: bitmap4,
        pub delegation: open_delegation4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct OPENATTR4args {
        pub createdir: bool,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct OPENATTR4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct OPEN_CONFIRM4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub open_stateid: stateid4<T>,
        pub seqid: seqid4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum OPEN_CONFIRM4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(OPEN_CONFIRM4resok<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct OPEN_CONFIRM4resok<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub open_stateid: stateid4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct OPEN_DOWNGRADE4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub open_stateid: stateid4<T>,
        pub seqid: seqid4,
        pub share_access: u32,
        pub share_deny: u32,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum OPEN_DOWNGRADE4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(OPEN_DOWNGRADE4resok<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct OPEN_DOWNGRADE4resok<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub open_stateid: stateid4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct PUTFH4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub object: nfs_fh4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct PUTFH4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct PUTPUBFH4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct PUTROOTFH4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct READ4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub stateid: stateid4<T>,
        pub offset: offset4,
        pub count: count4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum READ4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(READ4resok<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct READ4resok<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub eof: bool,
        pub data: T,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct READDIR4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub cookie: nfs_cookie4,
        pub cookieverf: verifier4<T>,
        pub dircount: count4,
        pub maxcount: count4,
        pub attr_request: bitmap4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum READDIR4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(READDIR4resok<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct READDIR4resok<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub cookieverf: verifier4<T>,
        pub reply: dirlist4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum READLINK4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(READLINK4resok<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct READLINK4resok<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub link: linktext4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct RELEASE_LOCKOWNER4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub lock_owner: lock_owner4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct RELEASE_LOCKOWNER4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct REMOVE4args {
        pub target: component4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum REMOVE4res {
        NFS4_OK(REMOVE4resok),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct REMOVE4resok {
        pub cinfo: change_info4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct RENAME4args {
        pub oldname: component4,
        pub newname: component4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum RENAME4res {
        NFS4_OK(RENAME4resok),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct RENAME4resok {
        pub source_cinfo: change_info4,
        pub target_cinfo: change_info4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct RENEW4args {
        pub clientid: clientid4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct RENEW4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct RESTOREFH4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct SAVEFH4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct SECINFO4args {
        pub name: component4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum SECINFO4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(SECINFO4resok<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct SECINFO4resok<T: AsRef<[u8]> + Debug>(pub Vec<secinfo4<T>>);
    #[derive(Debug, PartialEq, Clone)]
    pub struct SETATTR4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub stateid: stateid4<T>,
        pub obj_attributes: fattr4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct SETATTR4res {
        pub status: nfsstat4,
        pub attrsset: bitmap4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct SETCLIENTID4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub client: nfs_client_id4<T>,
        pub callback: cb_client4,
        pub callback_ident: u32,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum SETCLIENTID4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(SETCLIENTID4resok<T>),
        NFS4ERR_CLID_INUSE(clientaddr4),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct SETCLIENTID4resok<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub clientid: clientid4,
        pub setclientid_confirm: verifier4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct SETCLIENTID_CONFIRM4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub clientid: clientid4,
        pub setclientid_confirm: verifier4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct SETCLIENTID_CONFIRM4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct VERIFY4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub obj_attributes: fattr4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct VERIFY4res {
        pub status: nfsstat4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct WRITE4args<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub stateid: stateid4<T>,
        pub offset: offset4,
        pub stable: stable_how4,
        pub data: T,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum WRITE4res<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NFS4_OK(WRITE4resok<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct WRITE4resok<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub count: count4,
        pub committed: stable_how4,
        pub writeverf: verifier4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct aceflag4(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    pub struct acemask4(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    pub struct acetype4(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    pub struct ascii_REQUIRED4<T: AsRef<[u8]> + Debug>(pub utf8string<T>);
    #[derive(Debug, PartialEq, Clone)]
    pub struct attrlist4<T: AsRef<[u8]> + Debug>(pub T);
    #[derive(Debug, PartialEq, Eq, Hash, Clone)]
    pub struct bitmap4(pub Vec<bitmap_inner>);
    #[derive(Debug, PartialEq, Eq, Hash, Clone)]
    pub struct bitmap_inner(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    pub struct cb_client4 {
        pub cb_program: u32,
        pub cb_location: clientaddr4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct change_info4 {
        pub atomic: bool,
        pub before: changeid4,
        pub after: changeid4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct changeid4(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct clientaddr4 {
        pub r_netid: String,
        pub r_addr: String,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct clientid4(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct component4(pub utf8str_cs);
    #[derive(Debug, PartialEq, Clone)]
    pub struct count4(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    pub enum createhow4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        UNCHECKED4(fattr4<T>),
        GUARDED4(fattr4<T>),
        EXCLUSIVE4(verifier4<T>),
    }
    #[derive(Debug, PartialEq, Clone)]
    #[repr(u32)]
    pub enum createmode4 {
        UNCHECKED4 = 0,
        GUARDED4 = 1,
        EXCLUSIVE4 = 2,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum createtype4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        NF4LNK(linktext4<T>),
        NF4BLK(specdata4),
        NF4CHR(specdata4),
        NF4SOCK,
        NF4FIFO,
        NF4DIR,
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct dirlist4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub entries: Option<Box<entry4<T>>>,
        pub eof: bool,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct entry4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub cookie: nfs_cookie4,
        pub name: component4,
        pub attrs: fattr4<T>,
        pub nextentry: Option<Box<entry4<T>>>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct exist_lock_owner4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub lock_stateid: stateid4<T>,
        pub lock_seqid: seqid4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub attrmask: bitmap4,
        pub attr_vals: attrlist4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_acl<T: AsRef<[u8]> + Debug>(pub Vec<nfsace4<T>>);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_aclsupport(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_archive(pub bool);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_cansettime(pub bool);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_case_insensitive(pub bool);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_case_preserving(pub bool);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_change(pub changeid4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_chown_restricted(pub bool);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_fh_expire_type(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_filehandle<T: AsRef<[u8]> + Debug>(pub nfs_fh4<T>);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_fileid(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_files_avail(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_files_free(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_files_total(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_fs_locations(pub fs_locations4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_fsid(pub fsid4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_hidden(pub bool);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_homogeneous(pub bool);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_lease_time(pub nfs_lease4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_link_support(pub bool);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_maxfilesize(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_maxlink(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_maxname(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_maxread(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_maxwrite(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_mimetype<T: AsRef<[u8]> + Debug>(pub ascii_REQUIRED4<T>);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_mode(pub mode4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_mounted_on_fileid(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_named_attr(pub bool);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_no_trunc(pub bool);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_numlinks(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_owner<T: AsRef<[u8]> + Debug>(pub utf8str_mixed<T>);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_owner_group<T: AsRef<[u8]> + Debug>(pub utf8str_mixed<T>);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_quota_avail_hard(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_quota_avail_soft(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_quota_used(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_rawdev(pub specdata4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_rdattr_error(pub nfsstat4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_size(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_space_avail(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_space_free(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_space_total(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_space_used(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_supported_attrs(pub bitmap4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_symlink_support(pub bool);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_system(pub bool);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_time_access(pub nfstime4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_time_access_set(pub settime4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_time_backup(pub nfstime4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_time_create(pub nfstime4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_time_delta(pub nfstime4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_time_metadata(pub nfstime4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_time_modify(pub nfstime4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_time_modify_set(pub settime4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_type(pub nfs_ftype4);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fattr4_unique_handles(pub bool);
    #[derive(Debug, PartialEq, Clone)]
    pub struct fs_location4 {
        pub server: Vec<utf8str_cis>,
        pub rootpath: pathname4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct fs_locations4 {
        pub fs_root: pathname4,
        pub locations: Vec<fs_location4>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct fsid4 {
        pub major: u64,
        pub minor: u64,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct length4(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    #[repr(u32)]
    pub enum limit_by4 {
        NFS_LIMIT_SIZE = 1,
        NFS_LIMIT_BLOCKS = 2,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct linktext4<T: AsRef<[u8]> + Debug>(pub T);
    #[derive(Debug, PartialEq, Clone)]
    pub struct lock_owner4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub clientid: clientid4,
        pub owner: T,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum locker4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        TRUE(open_to_lock_owner4<T>),
        FALSE(exist_lock_owner4<T>),
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct mode4(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    pub enum nfs_argop4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        OP_ACCESS(ACCESS4args),
        OP_CLOSE(CLOSE4args<T>),
        OP_COMMIT(COMMIT4args),
        OP_CREATE(CREATE4args<T>),
        OP_DELEGPURGE(DELEGPURGE4args),
        OP_DELEGRETURN(DELEGRETURN4args<T>),
        OP_GETATTR(GETATTR4args),
        OP_LINK(LINK4args),
        OP_LOCK(LOCK4args<T>),
        OP_LOCKT(LOCKT4args<T>),
        OP_LOCKU(LOCKU4args<T>),
        OP_LOOKUP(LOOKUP4args),
        OP_NVERIFY(NVERIFY4args<T>),
        OP_OPEN(OPEN4args<T>),
        OP_OPENATTR(OPENATTR4args),
        OP_OPEN_CONFIRM(OPEN_CONFIRM4args<T>),
        OP_OPEN_DOWNGRADE(OPEN_DOWNGRADE4args<T>),
        OP_PUTFH(PUTFH4args<T>),
        OP_READ(READ4args<T>),
        OP_READDIR(READDIR4args<T>),
        OP_REMOVE(REMOVE4args),
        OP_RENAME(RENAME4args),
        OP_RENEW(RENEW4args),
        OP_SECINFO(SECINFO4args),
        OP_SETATTR(SETATTR4args<T>),
        OP_SETCLIENTID(SETCLIENTID4args<T>),
        OP_SETCLIENTID_CONFIRM(SETCLIENTID_CONFIRM4args<T>),
        OP_VERIFY(VERIFY4args<T>),
        OP_WRITE(WRITE4args<T>),
        OP_RELEASE_LOCKOWNER(RELEASE_LOCKOWNER4args<T>),
        OP_GETFH,
        OP_LOOKUPP,
        OP_PUTPUBFH,
        OP_PUTROOTFH,
        OP_READLINK,
        OP_RESTOREFH,
        OP_SAVEFH,
        OP_ILLEGAL,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum nfs_cb_argop4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        OP_CB_GETATTR(CB_GETATTR4args<T>),
        OP_CB_RECALL(CB_RECALL4args<T>),
        OP_CB_ILLEGAL,
    }
    #[derive(Debug, PartialEq, Clone)]
    #[repr(u32)]
    pub enum nfs_cb_opnum4 {
        OP_CB_GETATTR = 3,
        OP_CB_RECALL = 4,
        OP_CB_ILLEGAL = 10044,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum nfs_cb_resop4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        OP_CB_GETATTR(CB_GETATTR4res<T>),
        OP_CB_RECALL(CB_RECALL4res),
        OP_CB_ILLEGAL(CB_ILLEGAL4res),
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct nfs_client_id4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub verifier: verifier4<T>,
        pub id: T,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct nfs_cookie4(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct nfs_fh4<T: AsRef<[u8]> + Debug>(pub T);
    #[derive(Debug, PartialEq, Clone)]
    #[repr(u32)]
    pub enum nfs_ftype4 {
        NF4REG = 1,
        NF4DIR = 2,
        NF4BLK = 3,
        NF4CHR = 4,
        NF4LNK = 5,
        NF4SOCK = 6,
        NF4FIFO = 7,
        NF4ATTRDIR = 8,
        NF4NAMEDATTR = 9,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct nfs_lease4(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    #[repr(u32)]
    pub enum nfs_lock_type4 {
        READ_LT = 1,
        WRITE_LT = 2,
        READW_LT = 3,
        WRITEW_LT = 4,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct nfs_lockid4(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub struct nfs_modified_limit4 {
        pub num_blocks: u32,
        pub bytes_per_block: u32,
    }
    #[derive(Debug, PartialEq, Clone)]
    #[repr(u32)]
    pub enum nfs_opnum4 {
        OP_ACCESS = 3,
        OP_CLOSE = 4,
        OP_COMMIT = 5,
        OP_CREATE = 6,
        OP_DELEGPURGE = 7,
        OP_DELEGRETURN = 8,
        OP_GETATTR = 9,
        OP_GETFH = 10,
        OP_LINK = 11,
        OP_LOCK = 12,
        OP_LOCKT = 13,
        OP_LOCKU = 14,
        OP_LOOKUP = 15,
        OP_LOOKUPP = 16,
        OP_NVERIFY = 17,
        OP_OPEN = 18,
        OP_OPENATTR = 19,
        OP_OPEN_CONFIRM = 20,
        OP_OPEN_DOWNGRADE = 21,
        OP_PUTFH = 22,
        OP_PUTPUBFH = 23,
        OP_PUTROOTFH = 24,
        OP_READ = 25,
        OP_READDIR = 26,
        OP_READLINK = 27,
        OP_REMOVE = 28,
        OP_RENAME = 29,
        OP_RENEW = 30,
        OP_RESTOREFH = 31,
        OP_SAVEFH = 32,
        OP_SECINFO = 33,
        OP_SETATTR = 34,
        OP_SETCLIENTID = 35,
        OP_SETCLIENTID_CONFIRM = 36,
        OP_VERIFY = 37,
        OP_WRITE = 38,
        OP_RELEASE_LOCKOWNER = 39,
        OP_ILLEGAL = 10044,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum nfs_resop4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        OP_ACCESS(ACCESS4res),
        OP_CLOSE(CLOSE4res<T>),
        OP_COMMIT(COMMIT4res<T>),
        OP_CREATE(CREATE4res),
        OP_DELEGPURGE(DELEGPURGE4res),
        OP_DELEGRETURN(DELEGRETURN4res),
        OP_GETATTR(GETATTR4res<T>),
        OP_GETFH(GETFH4res<T>),
        OP_LINK(LINK4res),
        OP_LOCK(LOCK4res<T>),
        OP_LOCKT(LOCKT4res<T>),
        OP_LOCKU(LOCKU4res<T>),
        OP_LOOKUP(LOOKUP4res),
        OP_LOOKUPP(LOOKUPP4res),
        OP_NVERIFY(NVERIFY4res),
        OP_OPEN(OPEN4res<T>),
        OP_OPENATTR(OPENATTR4res),
        OP_OPEN_CONFIRM(OPEN_CONFIRM4res<T>),
        OP_OPEN_DOWNGRADE(OPEN_DOWNGRADE4res<T>),
        OP_PUTFH(PUTFH4res),
        OP_PUTPUBFH(PUTPUBFH4res),
        OP_PUTROOTFH(PUTROOTFH4res),
        OP_READ(READ4res<T>),
        OP_READDIR(READDIR4res<T>),
        OP_READLINK(READLINK4res<T>),
        OP_REMOVE(REMOVE4res),
        OP_RENAME(RENAME4res),
        OP_RENEW(RENEW4res),
        OP_RESTOREFH(RESTOREFH4res),
        OP_SAVEFH(SAVEFH4res),
        OP_SECINFO(SECINFO4res<T>),
        OP_SETATTR(SETATTR4res),
        OP_SETCLIENTID(SETCLIENTID4res<T>),
        OP_SETCLIENTID_CONFIRM(SETCLIENTID_CONFIRM4res),
        OP_VERIFY(VERIFY4res),
        OP_WRITE(WRITE4res<T>),
        OP_RELEASE_LOCKOWNER(RELEASE_LOCKOWNER4res),
        OP_ILLEGAL(ILLEGAL4res),
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum nfs_space_limit4 {
        NFS_LIMIT_SIZE(u64),
        NFS_LIMIT_BLOCKS(nfs_modified_limit4),
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct nfsace4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub type_v: acetype4,
        pub flag: aceflag4,
        pub access_mask: acemask4,
        pub who: utf8str_mixed<T>,
    }
    #[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
    pub enum nfsstat4 {
        NFS4_OK = 0,
        NFS4ERR_PERM = 1,
        NFS4ERR_NOENT = 2,
        NFS4ERR_IO = 5,
        NFS4ERR_NXIO = 6,
        NFS4ERR_ACCESS = 13,
        NFS4ERR_EXIST = 17,
        NFS4ERR_XDEV = 18,
        NFS4ERR_NOTDIR = 20,
        NFS4ERR_ISDIR = 21,
        NFS4ERR_INVAL = 22,
        NFS4ERR_FBIG = 27,
        NFS4ERR_NOSPC = 28,
        NFS4ERR_ROFS = 30,
        NFS4ERR_MLINK = 31,
        NFS4ERR_NAMETOOLONG = 63,
        NFS4ERR_NOTEMPTY = 66,
        NFS4ERR_DQUOT = 69,
        NFS4ERR_STALE = 70,
        NFS4ERR_BADHANDLE = 10001,
        NFS4ERR_BAD_COOKIE = 10003,
        NFS4ERR_NOTSUPP = 10004,
        NFS4ERR_TOOSMALL = 10005,
        NFS4ERR_SERVERFAULT = 10006,
        NFS4ERR_BADTYPE = 10007,
        NFS4ERR_DELAY = 10008,
        NFS4ERR_SAME = 10009,
        NFS4ERR_DENIED = 10010,
        NFS4ERR_EXPIRED = 10011,
        NFS4ERR_LOCKED = 10012,
        NFS4ERR_GRACE = 10013,
        NFS4ERR_FHEXPIRED = 10014,
        NFS4ERR_SHARE_DENIED = 10015,
        NFS4ERR_WRONGSEC = 10016,
        NFS4ERR_CLID_INUSE = 10017,
        NFS4ERR_RESOURCE = 10018,
        NFS4ERR_MOVED = 10019,
        NFS4ERR_NOFILEHANDLE = 10020,
        NFS4ERR_MINOR_VERS_MISMATCH = 10021,
        NFS4ERR_STALE_CLIENTID = 10022,
        NFS4ERR_STALE_STATEID = 10023,
        NFS4ERR_OLD_STATEID = 10024,
        NFS4ERR_BAD_STATEID = 10025,
        NFS4ERR_BAD_SEQID = 10026,
        NFS4ERR_NOT_SAME = 10027,
        NFS4ERR_LOCK_RANGE = 10028,
        NFS4ERR_SYMLINK = 10029,
        NFS4ERR_RESTOREFH = 10030,
        NFS4ERR_LEASE_MOVED = 10031,
        NFS4ERR_ATTRNOTSUPP = 10032,
        NFS4ERR_NO_GRACE = 10033,
        NFS4ERR_RECLAIM_BAD = 10034,
        NFS4ERR_RECLAIM_CONFLICT = 10035,
        NFS4ERR_BADXDR = 10036,
        NFS4ERR_LOCKS_HELD = 10037,
        NFS4ERR_OPENMODE = 10038,
        NFS4ERR_BADOWNER = 10039,
        NFS4ERR_BADCHAR = 10040,
        NFS4ERR_BADNAME = 10041,
        NFS4ERR_BAD_RANGE = 10042,
        NFS4ERR_LOCK_NOTSUPP = 10043,
        NFS4ERR_OP_ILLEGAL = 10044,
        NFS4ERR_DEADLOCK = 10045,
        NFS4ERR_FILE_OPEN = 10046,
        NFS4ERR_ADMIN_REVOKED = 10047,
        NFS4ERR_CB_PATH_DOWN = 10048,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct nfstime4 {
        pub seconds: i64,
        pub nseconds: u32,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct offset4(pub u64);
    #[derive(Debug, PartialEq, Clone)]
    pub enum open_claim4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        CLAIM_NULL(component4),
        CLAIM_PREVIOUS(open_delegation_type4),
        CLAIM_DELEGATE_CUR(open_claim_delegate_cur4<T>),
        CLAIM_DELEGATE_PREV(component4),
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct open_claim_delegate_cur4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub delegate_stateid: stateid4<T>,
        pub file: component4,
    }
    #[derive(Debug, PartialEq, Clone)]
    #[repr(u32)]
    pub enum open_claim_type4 {
        CLAIM_NULL = 0,
        CLAIM_PREVIOUS = 1,
        CLAIM_DELEGATE_CUR = 2,
        CLAIM_DELEGATE_PREV = 3,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum open_delegation4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        OPEN_DELEGATE_READ(open_read_delegation4<T>),
        OPEN_DELEGATE_WRITE(open_write_delegation4<T>),
        OPEN_DELEGATE_NONE,
    }
    #[derive(Debug, PartialEq, Clone)]
    #[repr(u32)]
    pub enum open_delegation_type4 {
        OPEN_DELEGATE_NONE = 0,
        OPEN_DELEGATE_READ = 1,
        OPEN_DELEGATE_WRITE = 2,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct open_owner4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub clientid: clientid4,
        pub owner: T,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct open_read_delegation4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub stateid: stateid4<T>,
        pub recall: bool,
        pub permissions: nfsace4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct open_to_lock_owner4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub open_seqid: seqid4,
        pub open_stateid: stateid4<T>,
        pub lock_seqid: seqid4,
        pub lock_owner: lock_owner4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct open_write_delegation4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub stateid: stateid4<T>,
        pub recall: bool,
        pub space_limit: nfs_space_limit4,
        pub permissions: nfsace4<T>,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub enum openflag4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        OPEN4_NOCREATE,
        OPEN4_CREATE(createhow4<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    #[repr(u32)]
    pub enum opentype4 {
        OPEN4_NOCREATE = 0,
        OPEN4_CREATE = 1,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct pathname4(pub Vec<component4>);
    #[derive(Debug, PartialEq, Clone)]
    pub struct qop4(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    #[repr(u32)]
    pub enum rpc_gss_svc_t {
        RPC_GSS_SVC_NONE = 1,
        RPC_GSS_SVC_INTEGRITY = 2,
        RPC_GSS_SVC_PRIVACY = 3,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct rpcsec_gss_info<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub oid: sec_oid4<T>,
        pub qop: qop4,
        pub service: rpc_gss_svc_t,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct sec_oid4<T: AsRef<[u8]> + Debug>(pub T);
    #[derive(Debug, PartialEq, Clone)]
    pub enum secinfo4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        RPCSEC_GSS(rpcsec_gss_info<T>),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct seqid4(pub u32);
    #[derive(Debug, PartialEq, Clone)]
    pub enum settime4 {
        SET_TO_CLIENT_TIME4(nfstime4),
        default,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct specdata4 {
        pub specdata1: u32,
        pub specdata2: u32,
    }
    #[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
    pub enum stable_how4 {

        UNSTABLE4 = 0,
        DATA_SYNC4 = 1,
        FILE_SYNC4 = 2,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct stateid4<T>
    where
        T: AsRef<[u8]> + Debug,
    {
        pub seqid: u32,
        pub other: T,
    }
    #[derive(Debug, PartialEq, Clone)]
    #[repr(u32)]
    pub enum time_how4 {
        SET_TO_SERVER_TIME4 = 0,
        SET_TO_CLIENT_TIME4 = 1,
    }
    #[derive(Debug, PartialEq, Clone)]
    pub struct utf8str_cis(pub String);
    #[derive(Debug, PartialEq, Clone)]
    pub struct utf8str_cs(pub String);
    #[derive(Debug, PartialEq, Clone)]
    pub struct utf8str_mixed<T: AsRef<[u8]> + Debug>(pub utf8string<T>);
    #[derive(Debug, PartialEq, Clone)]
    pub struct utf8string<T: AsRef<[u8]> + Debug>(pub T);
    #[derive(Debug, PartialEq, Clone)]
    pub struct verifier4<T: AsRef<[u8]> + Debug>(pub T);
    impl Serialisable for u32 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(*self)
        }
    }
    impl Serialisable for i32 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_i32(*self)
        }
    }
    impl Serialisable for f32 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_f32(*self)
        }
    }
    impl Serialisable for u64 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u64(*self)
        }
    }
    impl Serialisable for i64 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_i64(*self)
        }
    }
    impl Serialisable for f64 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_f64(*self)
        }
    }
    impl Serialisable for bool {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(*self as u32)
        }
    }
    impl Serialisable for String {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.len() as u32);
            dst.put_slice(self.as_bytes());
            for _ in 0..pad_length(self.len()) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for Vec<T>
    where
        T: Serialisable,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.len() as u32);
            let n_start = dst.len();
            self.iter().for_each(|i| i.serialize(dst));
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for [T]
    where
        T: Serialisable,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.iter().for_each(|i| i.serialize(dst));
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for Option<Box<T>>
    where
        T: Serialisable,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.is_some() as u32);
            if let Some(v) = self {
                v.serialize(dst)
            }
        }
    }
    impl Serialisable for Bytes {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_slice(self.as_ref())
        }
    }
    impl Serialisable for ACCESS4args {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.access.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for ACCESS4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for ACCESS4resok {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.supported.serialize(dst);
            self.access.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for CB_COMPOUND4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.tag.serialize(dst);
            self.minorversion.serialize(dst);
            self.callback_ident.serialize(dst);
            self.argarray.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for CB_COMPOUND4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            self.tag.serialize(dst);
            self.resarray.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for CB_GETATTR4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.fh.serialize(dst);
            self.attr_request.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for CB_GETATTR4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for CB_GETATTR4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.obj_attributes.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for CB_ILLEGAL4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for CB_RECALL4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.stateid.serialize(dst);
            self.truncate.serialize(dst);
            self.fh.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for CB_RECALL4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for CLOSE4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.seqid.serialize(dst);
            self.open_stateid.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for CLOSE4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for COMMIT4args {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.offset.serialize(dst);
            self.count.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for COMMIT4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for COMMIT4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.writeverf.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for COMPOUND4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.tag.serialize(dst);
            self.minorversion.serialize(dst);
            self.argarray.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for COMPOUND4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            self.tag.serialize(dst);
            self.resarray.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for CREATE4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.objtype.serialize(dst);
            self.objname.serialize(dst);
            self.createattrs.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for CREATE4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for CREATE4resok {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.cinfo.serialize(dst);
            self.attrset.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for DELEGPURGE4args {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.clientid.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for DELEGPURGE4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for DELEGRETURN4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.deleg_stateid.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for DELEGRETURN4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for GETATTR4args {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.attr_request.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for GETATTR4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for GETATTR4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.obj_attributes.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for GETFH4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for GETFH4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.object.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for ILLEGAL4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for LINK4args {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.newname.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for LINK4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for LINK4resok {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.cinfo.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for LOCK4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.locktype.serialize(dst);
            self.reclaim.serialize(dst);
            self.offset.serialize(dst);
            self.length.serialize(dst);
            self.locker.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for LOCK4denied<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.offset.serialize(dst);
            self.length.serialize(dst);
            self.locktype.serialize(dst);
            self.owner.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for LOCK4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::NFS4ERR_DENIED(var) => {
                    dst.put_u32(nfsstat4::NFS4ERR_DENIED as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for LOCK4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.lock_stateid.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for LOCKT4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.locktype.serialize(dst);
            self.offset.serialize(dst);
            self.length.serialize(dst);
            self.owner.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for LOCKT4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4ERR_DENIED(var) => {
                    dst.put_u32(nfsstat4::NFS4ERR_DENIED as u32);
                    var.serialize(dst);
                }
                Self::NFS4_OK => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for LOCKU4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.locktype.serialize(dst);
            self.seqid.serialize(dst);
            self.lock_stateid.serialize(dst);
            self.offset.serialize(dst);
            self.length.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for LOCKU4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for LOOKUP4args {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.objname.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for LOOKUP4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for LOOKUPP4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for NVERIFY4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.obj_attributes.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for NVERIFY4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for OPEN4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.seqid.serialize(dst);
            self.share_access.serialize(dst);
            self.share_deny.serialize(dst);
            self.owner.serialize(dst);
            self.openhow.serialize(dst);
            self.claim.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for OPEN4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for OPEN4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.stateid.serialize(dst);
            self.cinfo.serialize(dst);
            self.rflags.serialize(dst);
            self.attrset.serialize(dst);
            self.delegation.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for OPENATTR4args {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.createdir.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for OPENATTR4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for OPEN_CONFIRM4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.open_stateid.serialize(dst);
            self.seqid.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for OPEN_CONFIRM4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for OPEN_CONFIRM4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.open_stateid.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for OPEN_DOWNGRADE4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.open_stateid.serialize(dst);
            self.seqid.serialize(dst);
            self.share_access.serialize(dst);
            self.share_deny.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for OPEN_DOWNGRADE4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for OPEN_DOWNGRADE4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.open_stateid.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for PUTFH4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.object.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for PUTFH4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for PUTPUBFH4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for PUTROOTFH4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for READ4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.stateid.serialize(dst);
            self.offset.serialize(dst);
            self.count.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for READ4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for READ4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.eof.serialize(dst);
            dst.put_u32(self.data.as_ref().len() as u32);
            self.data.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for READDIR4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.cookie.serialize(dst);
            self.cookieverf.serialize(dst);
            self.dircount.serialize(dst);
            self.maxcount.serialize(dst);
            self.attr_request.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for READDIR4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for READDIR4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.cookieverf.serialize(dst);
            self.reply.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for READLINK4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for READLINK4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.link.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for RELEASE_LOCKOWNER4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.lock_owner.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for RELEASE_LOCKOWNER4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for REMOVE4args {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.target.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for REMOVE4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for REMOVE4resok {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.cinfo.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for RENAME4args {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.oldname.serialize(dst);
            self.newname.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for RENAME4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for RENAME4resok {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.source_cinfo.serialize(dst);
            self.target_cinfo.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for RENEW4args {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.clientid.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for RENEW4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for RESTOREFH4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for SAVEFH4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for SECINFO4args {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.name.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for SECINFO4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for SECINFO4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl<T> Serialisable for SETATTR4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.stateid.serialize(dst);
            self.obj_attributes.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for SETATTR4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            self.attrsset.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for SETCLIENTID4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.client.serialize(dst);
            self.callback.serialize(dst);
            self.callback_ident.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for SETCLIENTID4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::NFS4ERR_CLID_INUSE(var) => {
                    dst.put_u32(nfsstat4::NFS4ERR_CLID_INUSE as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for SETCLIENTID4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.clientid.serialize(dst);
            self.setclientid_confirm.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for SETCLIENTID_CONFIRM4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.clientid.serialize(dst);
            self.setclientid_confirm.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for SETCLIENTID_CONFIRM4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for VERIFY4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.obj_attributes.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for VERIFY4res {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.status.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for WRITE4args<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.stateid.serialize(dst);
            self.offset.serialize(dst);
            self.stable.serialize(dst);
            self.data.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for WRITE4res<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS4_OK(var) => {
                    dst.put_u32(nfsstat4::NFS4_OK as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for WRITE4resok<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.count.serialize(dst);
            self.committed.serialize(dst);
            self.writeverf.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for aceflag4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for acemask4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for acetype4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl<T> Serialisable for ascii_REQUIRED4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl<T> Serialisable for attrlist4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.0.as_ref().len() as u32);
            dst.put_slice(self.0.as_ref());
            for _ in 0..pad_length(self.0.as_ref().len()) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for bitmap4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for bitmap_inner {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for cb_client4 {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.cb_program.serialize(dst);
            self.cb_location.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for change_info4 {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.atomic.serialize(dst);
            self.before.serialize(dst);
            self.after.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for changeid4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for clientaddr4 {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.r_netid.serialize(dst);
            self.r_addr.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for clientid4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for component4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for count4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl<T> Serialisable for createhow4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::UNCHECKED4(var) => {
                    dst.put_u32(createmode4::UNCHECKED4 as u32);
                    var.serialize(dst);
                }
                Self::GUARDED4(var) => {
                    dst.put_u32(createmode4::GUARDED4 as u32);
                    var.serialize(dst);
                }
                Self::EXCLUSIVE4(var) => {
                    dst.put_u32(createmode4::EXCLUSIVE4 as u32);
                    var.serialize(dst);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for createmode4 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.clone() as u32);
        }
    }
    impl<T> Serialisable for createtype4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NF4LNK(var) => {
                    dst.put_u32(nfs_ftype4::NF4LNK as u32);
                    var.serialize(dst);
                }
                Self::NF4BLK(var) => {
                    dst.put_u32(nfs_ftype4::NF4BLK as u32);
                    var.serialize(dst);
                }
                Self::NF4CHR(var) => {
                    dst.put_u32(nfs_ftype4::NF4CHR as u32);
                    var.serialize(dst);
                }
                Self::NF4SOCK => {
                    dst.put_u32(nfs_ftype4::NF4SOCK as u32);
                }
                Self::NF4FIFO => {
                    dst.put_u32(nfs_ftype4::NF4FIFO as u32);
                }
                Self::NF4DIR => {
                    dst.put_u32(nfs_ftype4::NF4DIR as u32);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for dirlist4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.entries.serialize(dst);
            self.eof.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for entry4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.cookie.serialize(dst);
            self.name.serialize(dst);
            self.attrs.serialize(dst);
            self.nextentry.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for exist_lock_owner4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.lock_stateid.serialize(dst);
            self.lock_seqid.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for fattr4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.attrmask.serialize(dst);
            self.attr_vals.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for fattr4_acl<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_aclsupport {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_archive {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_cansettime {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_case_insensitive {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_case_preserving {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_change {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_chown_restricted {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_fh_expire_type {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl<T> Serialisable for fattr4_filehandle<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_fileid {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_files_avail {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_files_free {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_files_total {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_fs_locations {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_fsid {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_hidden {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_homogeneous {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_lease_time {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_link_support {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_maxfilesize {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_maxlink {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_maxname {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_maxread {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_maxwrite {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl<T> Serialisable for fattr4_mimetype<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_mode {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_mounted_on_fileid {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_named_attr {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_no_trunc {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_numlinks {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl<T> Serialisable for fattr4_owner<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl<T> Serialisable for fattr4_owner_group<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_quota_avail_hard {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_quota_avail_soft {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_quota_used {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_rawdev {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_rdattr_error {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_size {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_space_avail {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_space_free {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_space_total {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_space_used {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_supported_attrs {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_symlink_support {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_system {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_time_access {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_time_access_set {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_time_backup {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_time_create {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_time_delta {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_time_metadata {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_time_modify {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_time_modify_set {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_type {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fattr4_unique_handles {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for fs_location4 {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.server.serialize(dst);
            self.rootpath.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for fs_locations4 {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.fs_root.serialize(dst);
            self.locations.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for fsid4 {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.major.serialize(dst);
            self.minor.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for length4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for limit_by4 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.clone() as u32);
        }
    }
    impl<T> Serialisable for linktext4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.0.as_ref().len() as u32);
            dst.put_slice(self.0.as_ref());
            for _ in 0..pad_length(self.0.as_ref().len()) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for lock_owner4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.clientid.serialize(dst);
            self.owner.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for locker4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::TRUE(var) => {
                    dst.put_u32(true as u32);
                    var.serialize(dst);
                }
                Self::FALSE(var) => {
                    dst.put_u32(false as u32);
                    var.serialize(dst);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for mode4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl<T> Serialisable for nfs_argop4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::OP_ACCESS(var) => {
                    dst.put_u32(nfs_opnum4::OP_ACCESS as u32);
                    var.serialize(dst);
                }
                Self::OP_CLOSE(var) => {
                    dst.put_u32(nfs_opnum4::OP_CLOSE as u32);
                    var.serialize(dst);
                }
                Self::OP_COMMIT(var) => {
                    dst.put_u32(nfs_opnum4::OP_COMMIT as u32);
                    var.serialize(dst);
                }
                Self::OP_CREATE(var) => {
                    dst.put_u32(nfs_opnum4::OP_CREATE as u32);
                    var.serialize(dst);
                }
                Self::OP_DELEGPURGE(var) => {
                    dst.put_u32(nfs_opnum4::OP_DELEGPURGE as u32);
                    var.serialize(dst);
                }
                Self::OP_DELEGRETURN(var) => {
                    dst.put_u32(nfs_opnum4::OP_DELEGRETURN as u32);
                    var.serialize(dst);
                }
                Self::OP_GETATTR(var) => {
                    dst.put_u32(nfs_opnum4::OP_GETATTR as u32);
                    var.serialize(dst);
                }
                Self::OP_LINK(var) => {
                    dst.put_u32(nfs_opnum4::OP_LINK as u32);
                    var.serialize(dst);
                }
                Self::OP_LOCK(var) => {
                    dst.put_u32(nfs_opnum4::OP_LOCK as u32);
                    var.serialize(dst);
                }
                Self::OP_LOCKT(var) => {
                    dst.put_u32(nfs_opnum4::OP_LOCKT as u32);
                    var.serialize(dst);
                }
                Self::OP_LOCKU(var) => {
                    dst.put_u32(nfs_opnum4::OP_LOCKU as u32);
                    var.serialize(dst);
                }
                Self::OP_LOOKUP(var) => {
                    dst.put_u32(nfs_opnum4::OP_LOOKUP as u32);
                    var.serialize(dst);
                }
                Self::OP_NVERIFY(var) => {
                    dst.put_u32(nfs_opnum4::OP_NVERIFY as u32);
                    var.serialize(dst);
                }
                Self::OP_OPEN(var) => {
                    dst.put_u32(nfs_opnum4::OP_OPEN as u32);
                    var.serialize(dst);
                }
                Self::OP_OPENATTR(var) => {
                    dst.put_u32(nfs_opnum4::OP_OPENATTR as u32);
                    var.serialize(dst);
                }
                Self::OP_OPEN_CONFIRM(var) => {
                    dst.put_u32(nfs_opnum4::OP_OPEN_CONFIRM as u32);
                    var.serialize(dst);
                }
                Self::OP_OPEN_DOWNGRADE(var) => {
                    dst.put_u32(nfs_opnum4::OP_OPEN_DOWNGRADE as u32);
                    var.serialize(dst);
                }
                Self::OP_PUTFH(var) => {
                    dst.put_u32(nfs_opnum4::OP_PUTFH as u32);
                    var.serialize(dst);
                }
                Self::OP_READ(var) => {
                    dst.put_u32(nfs_opnum4::OP_READ as u32);
                    var.serialize(dst);
                }
                Self::OP_READDIR(var) => {
                    dst.put_u32(nfs_opnum4::OP_READDIR as u32);
                    var.serialize(dst);
                }
                Self::OP_REMOVE(var) => {
                    dst.put_u32(nfs_opnum4::OP_REMOVE as u32);
                    var.serialize(dst);
                }
                Self::OP_RENAME(var) => {
                    dst.put_u32(nfs_opnum4::OP_RENAME as u32);
                    var.serialize(dst);
                }
                Self::OP_RENEW(var) => {
                    dst.put_u32(nfs_opnum4::OP_RENEW as u32);
                    var.serialize(dst);
                }
                Self::OP_SECINFO(var) => {
                    dst.put_u32(nfs_opnum4::OP_SECINFO as u32);
                    var.serialize(dst);
                }
                Self::OP_SETATTR(var) => {
                    dst.put_u32(nfs_opnum4::OP_SETATTR as u32);
                    var.serialize(dst);
                }
                Self::OP_SETCLIENTID(var) => {
                    dst.put_u32(nfs_opnum4::OP_SETCLIENTID as u32);
                    var.serialize(dst);
                }
                Self::OP_SETCLIENTID_CONFIRM(var) => {
                    dst.put_u32(nfs_opnum4::OP_SETCLIENTID_CONFIRM as u32);
                    var.serialize(dst);
                }
                Self::OP_VERIFY(var) => {
                    dst.put_u32(nfs_opnum4::OP_VERIFY as u32);
                    var.serialize(dst);
                }
                Self::OP_WRITE(var) => {
                    dst.put_u32(nfs_opnum4::OP_WRITE as u32);
                    var.serialize(dst);
                }
                Self::OP_RELEASE_LOCKOWNER(var) => {
                    dst.put_u32(nfs_opnum4::OP_RELEASE_LOCKOWNER as u32);
                    var.serialize(dst);
                }
                Self::OP_GETFH => {
                    dst.put_u32(nfs_opnum4::OP_GETFH as u32);
                }
                Self::OP_LOOKUPP => {
                    dst.put_u32(nfs_opnum4::OP_LOOKUPP as u32);
                }
                Self::OP_PUTPUBFH => {
                    dst.put_u32(nfs_opnum4::OP_PUTPUBFH as u32);
                }
                Self::OP_PUTROOTFH => {
                    dst.put_u32(nfs_opnum4::OP_PUTROOTFH as u32);
                }
                Self::OP_READLINK => {
                    dst.put_u32(nfs_opnum4::OP_READLINK as u32);
                }
                Self::OP_RESTOREFH => {
                    dst.put_u32(nfs_opnum4::OP_RESTOREFH as u32);
                }
                Self::OP_SAVEFH => {
                    dst.put_u32(nfs_opnum4::OP_SAVEFH as u32);
                }
                Self::OP_ILLEGAL => {
                    dst.put_u32(nfs_opnum4::OP_ILLEGAL as u32);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for nfs_cb_argop4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::OP_CB_GETATTR(var) => {
                    dst.put_u32(nfs_cb_opnum4::OP_CB_GETATTR as u32);
                    var.serialize(dst);
                }
                Self::OP_CB_RECALL(var) => {
                    dst.put_u32(nfs_cb_opnum4::OP_CB_RECALL as u32);
                    var.serialize(dst);
                }
                Self::OP_CB_ILLEGAL => {
                    dst.put_u32(nfs_cb_opnum4::OP_CB_ILLEGAL as u32);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for nfs_cb_opnum4 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.clone() as u32);
        }
    }
    impl<T> Serialisable for nfs_cb_resop4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::OP_CB_GETATTR(var) => {
                    dst.put_u32(nfs_cb_opnum4::OP_CB_GETATTR as u32);
                    var.serialize(dst);
                }
                Self::OP_CB_RECALL(var) => {
                    dst.put_u32(nfs_cb_opnum4::OP_CB_RECALL as u32);
                    var.serialize(dst);
                }
                Self::OP_CB_ILLEGAL(var) => {
                    dst.put_u32(nfs_cb_opnum4::OP_CB_ILLEGAL as u32);
                    var.serialize(dst);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for nfs_client_id4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.verifier.serialize(dst);
            self.id.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for nfs_cookie4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl<T> Serialisable for nfs_fh4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.0.as_ref().len() as u32);
            dst.put_slice(self.0.as_ref());
            for _ in 0..pad_length(self.0.as_ref().len()) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for nfs_ftype4 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.clone() as u32);
        }
    }
    impl Serialisable for nfs_lease4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for nfs_lock_type4 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.clone() as u32);
        }
    }
    impl Serialisable for nfs_lockid4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for nfs_modified_limit4 {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.num_blocks.serialize(dst);
            self.bytes_per_block.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for nfs_opnum4 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.clone() as u32);
        }
    }
    impl<T> Serialisable for nfs_resop4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::OP_ACCESS(var) => {
                    dst.put_u32(nfs_opnum4::OP_ACCESS as u32);
                    var.serialize(dst);
                }
                Self::OP_CLOSE(var) => {
                    dst.put_u32(nfs_opnum4::OP_CLOSE as u32);
                    var.serialize(dst);
                }
                Self::OP_COMMIT(var) => {
                    dst.put_u32(nfs_opnum4::OP_COMMIT as u32);
                    var.serialize(dst);
                }
                Self::OP_CREATE(var) => {
                    dst.put_u32(nfs_opnum4::OP_CREATE as u32);
                    var.serialize(dst);
                }
                Self::OP_DELEGPURGE(var) => {
                    dst.put_u32(nfs_opnum4::OP_DELEGPURGE as u32);
                    var.serialize(dst);
                }
                Self::OP_DELEGRETURN(var) => {
                    dst.put_u32(nfs_opnum4::OP_DELEGRETURN as u32);
                    var.serialize(dst);
                }
                Self::OP_GETATTR(var) => {
                    dst.put_u32(nfs_opnum4::OP_GETATTR as u32);
                    var.serialize(dst);
                }
                Self::OP_GETFH(var) => {
                    dst.put_u32(nfs_opnum4::OP_GETFH as u32);
                    var.serialize(dst);
                }
                Self::OP_LINK(var) => {
                    dst.put_u32(nfs_opnum4::OP_LINK as u32);
                    var.serialize(dst);
                }
                Self::OP_LOCK(var) => {
                    dst.put_u32(nfs_opnum4::OP_LOCK as u32);
                    var.serialize(dst);
                }
                Self::OP_LOCKT(var) => {
                    dst.put_u32(nfs_opnum4::OP_LOCKT as u32);
                    var.serialize(dst);
                }
                Self::OP_LOCKU(var) => {
                    dst.put_u32(nfs_opnum4::OP_LOCKU as u32);
                    var.serialize(dst);
                }
                Self::OP_LOOKUP(var) => {
                    dst.put_u32(nfs_opnum4::OP_LOOKUP as u32);
                    var.serialize(dst);
                }
                Self::OP_LOOKUPP(var) => {
                    dst.put_u32(nfs_opnum4::OP_LOOKUPP as u32);
                    var.serialize(dst);
                }
                Self::OP_NVERIFY(var) => {
                    dst.put_u32(nfs_opnum4::OP_NVERIFY as u32);
                    var.serialize(dst);
                }
                Self::OP_OPEN(var) => {
                    dst.put_u32(nfs_opnum4::OP_OPEN as u32);
                    var.serialize(dst);
                }
                Self::OP_OPENATTR(var) => {
                    dst.put_u32(nfs_opnum4::OP_OPENATTR as u32);
                    var.serialize(dst);
                }
                Self::OP_OPEN_CONFIRM(var) => {
                    dst.put_u32(nfs_opnum4::OP_OPEN_CONFIRM as u32);
                    var.serialize(dst);
                }
                Self::OP_OPEN_DOWNGRADE(var) => {
                    dst.put_u32(nfs_opnum4::OP_OPEN_DOWNGRADE as u32);
                    var.serialize(dst);
                }
                Self::OP_PUTFH(var) => {
                    dst.put_u32(nfs_opnum4::OP_PUTFH as u32);
                    var.serialize(dst);
                }
                Self::OP_PUTPUBFH(var) => {
                    dst.put_u32(nfs_opnum4::OP_PUTPUBFH as u32);
                    var.serialize(dst);
                }
                Self::OP_PUTROOTFH(var) => {
                    dst.put_u32(nfs_opnum4::OP_PUTROOTFH as u32);
                    var.serialize(dst);
                }
                Self::OP_READ(var) => {
                    dst.put_u32(nfs_opnum4::OP_READ as u32);
                    var.serialize(dst);
                }
                Self::OP_READDIR(var) => {
                    dst.put_u32(nfs_opnum4::OP_READDIR as u32);
                    var.serialize(dst);
                }
                Self::OP_READLINK(var) => {
                    dst.put_u32(nfs_opnum4::OP_READLINK as u32);
                    var.serialize(dst);
                }
                Self::OP_REMOVE(var) => {
                    dst.put_u32(nfs_opnum4::OP_REMOVE as u32);
                    var.serialize(dst);
                }
                Self::OP_RENAME(var) => {
                    dst.put_u32(nfs_opnum4::OP_RENAME as u32);
                    var.serialize(dst);
                }
                Self::OP_RENEW(var) => {
                    dst.put_u32(nfs_opnum4::OP_RENEW as u32);
                    var.serialize(dst);
                }
                Self::OP_RESTOREFH(var) => {
                    dst.put_u32(nfs_opnum4::OP_RESTOREFH as u32);
                    var.serialize(dst);
                }
                Self::OP_SAVEFH(var) => {
                    dst.put_u32(nfs_opnum4::OP_SAVEFH as u32);
                    var.serialize(dst);
                }
                Self::OP_SECINFO(var) => {
                    dst.put_u32(nfs_opnum4::OP_SECINFO as u32);
                    var.serialize(dst);
                }
                Self::OP_SETATTR(var) => {
                    dst.put_u32(nfs_opnum4::OP_SETATTR as u32);
                    var.serialize(dst);
                }
                Self::OP_SETCLIENTID(var) => {
                    dst.put_u32(nfs_opnum4::OP_SETCLIENTID as u32);
                    var.serialize(dst);
                }
                Self::OP_SETCLIENTID_CONFIRM(var) => {
                    dst.put_u32(nfs_opnum4::OP_SETCLIENTID_CONFIRM as u32);
                    var.serialize(dst);
                }
                Self::OP_VERIFY(var) => {
                    dst.put_u32(nfs_opnum4::OP_VERIFY as u32);
                    var.serialize(dst);
                }
                Self::OP_WRITE(var) => {
                    dst.put_u32(nfs_opnum4::OP_WRITE as u32);
                    var.serialize(dst);
                }
                Self::OP_RELEASE_LOCKOWNER(var) => {
                    dst.put_u32(nfs_opnum4::OP_RELEASE_LOCKOWNER as u32);
                    var.serialize(dst);
                }
                Self::OP_ILLEGAL(var) => {
                    dst.put_u32(nfs_opnum4::OP_ILLEGAL as u32);
                    var.serialize(dst);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for nfs_space_limit4 {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::NFS_LIMIT_SIZE(var) => {
                    dst.put_u32(limit_by4::NFS_LIMIT_SIZE as u32);
                    var.serialize(dst);
                }
                Self::NFS_LIMIT_BLOCKS(var) => {
                    dst.put_u32(limit_by4::NFS_LIMIT_BLOCKS as u32);
                    var.serialize(dst);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for nfsace4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.type_v.serialize(dst);
            self.flag.serialize(dst);
            self.access_mask.serialize(dst);
            self.who.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for nfsstat4 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.clone() as u32);
        }
    }
    impl Serialisable for nfstime4 {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.seconds.serialize(dst);
            self.nseconds.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for offset4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl<T> Serialisable for open_claim4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::CLAIM_NULL(var) => {
                    dst.put_u32(open_claim_type4::CLAIM_NULL as u32);
                    var.serialize(dst);
                }
                Self::CLAIM_PREVIOUS(var) => {
                    dst.put_u32(open_claim_type4::CLAIM_PREVIOUS as u32);
                    var.serialize(dst);
                }
                Self::CLAIM_DELEGATE_CUR(var) => {
                    dst.put_u32(open_claim_type4::CLAIM_DELEGATE_CUR as u32);
                    var.serialize(dst);
                }
                Self::CLAIM_DELEGATE_PREV(var) => {
                    dst.put_u32(open_claim_type4::CLAIM_DELEGATE_PREV as u32);
                    var.serialize(dst);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for open_claim_delegate_cur4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.delegate_stateid.serialize(dst);
            self.file.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for open_claim_type4 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.clone() as u32);
        }
    }
    impl<T> Serialisable for open_delegation4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::OPEN_DELEGATE_READ(var) => {
                    dst.put_u32(open_delegation_type4::OPEN_DELEGATE_READ as u32);
                    var.serialize(dst);
                }
                Self::OPEN_DELEGATE_WRITE(var) => {
                    dst.put_u32(open_delegation_type4::OPEN_DELEGATE_WRITE as u32);
                    var.serialize(dst);
                }
                Self::OPEN_DELEGATE_NONE => {
                    dst.put_u32(open_delegation_type4::OPEN_DELEGATE_NONE as u32);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for open_delegation_type4 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.clone() as u32);
        }
    }
    impl<T> Serialisable for open_owner4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.clientid.serialize(dst);
            self.owner.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for open_read_delegation4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.stateid.serialize(dst);
            self.recall.serialize(dst);
            self.permissions.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for open_to_lock_owner4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.open_seqid.serialize(dst);
            self.open_stateid.serialize(dst);
            self.lock_seqid.serialize(dst);
            self.lock_owner.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for open_write_delegation4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.stateid.serialize(dst);
            self.recall.serialize(dst);
            self.space_limit.serialize(dst);
            self.permissions.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for openflag4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::OPEN4_CREATE(var) => {
                    dst.put_u32(opentype4::OPEN4_CREATE as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for opentype4 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.clone() as u32);
        }
    }
    impl Serialisable for pathname4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for qop4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for rpc_gss_svc_t {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.clone() as u32);
        }
    }
    impl<T> Serialisable for rpcsec_gss_info<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.oid.serialize(dst);
            self.qop.serialize(dst);
            self.service.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for sec_oid4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.0.as_ref().len() as u32);
            dst.put_slice(self.0.as_ref());
            for _ in 0..pad_length(self.0.as_ref().len()) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for secinfo4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::RPCSEC_GSS(var) => {
                    dst.put_u32(6);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for seqid4 {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for settime4 {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            match self {
                Self::SET_TO_CLIENT_TIME4(var) => {
                    dst.put_u32(time_how4::SET_TO_CLIENT_TIME4 as u32);
                    var.serialize(dst);
                }
                Self::default => {
                    dst.put_u32(0);
                }
                _ => dst.put_i32(0),
            }
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for specdata4 {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.specdata1.serialize(dst);
            self.specdata2.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for stable_how4 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.clone() as u32);
        }
    }
    impl<T> Serialisable for stateid4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            let n_start = dst.len();
            self.seqid.serialize(dst);
            self.other.serialize(dst);
            for _ in 0..pad_length(dst.len() - n_start) {
                dst.put_u8(0)
            }
        }
    }
    impl Serialisable for time_how4 {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.clone() as u32);
        }
    }
    impl Serialisable for utf8str_cis {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl Serialisable for utf8str_cs {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl<T> Serialisable for utf8str_mixed<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            self.0.serialize(dst);
        }
    }
    impl<T> Serialisable for utf8string<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_u32(self.0.as_ref().len() as u32);
            dst.put_slice(self.0.as_ref());
            for _ in 0..pad_length(self.0.as_ref().len()) {
                dst.put_u8(0)
            }
        }
    }
    impl<T> Serialisable for verifier4<T>
    where
        T: Serialisable + AsRef<[u8]> + Debug,
    {
        fn serialize(&self, dst: &mut BytesMut) {
            dst.put_slice(self.0.as_ref());
            for _ in 0..pad_length(self.0.as_ref().len()) {
                dst.put_u8(0)
            }
        }
    }
    impl TryFrom<Bytes> for ACCESS4args {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(ACCESS4args {
                access: v.read_u32()?,
            })
        }
    }
    impl TryFrom<Bytes> for ACCESS4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(ACCESS4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for ACCESS4resok {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(ACCESS4resok {
                supported: v.read_u32()?,
                access: v.read_u32()?,
            })
        }
    }
    impl TryFrom<Bytes> for CB_COMPOUND4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(CB_COMPOUND4args {
                tag: utf8str_cs::try_from(&mut v)?,
                minorversion: v.read_u32()?,
                callback_ident: v.read_u32()?,
                argarray: v.read_variable_array::<nfs_cb_argop4<Bytes>>(None)?,
            })
        }
    }
    impl TryFrom<Bytes> for CB_COMPOUND4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(CB_COMPOUND4res {
                status: nfsstat4::try_from(&mut v)?,
                tag: utf8str_cs::try_from(&mut v)?,
                resarray: v.read_variable_array::<nfs_cb_resop4<Bytes>>(None)?,
            })
        }
    }
    impl TryFrom<Bytes> for CB_GETATTR4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(CB_GETATTR4args {
                fh: nfs_fh4::try_from(&mut v)?,
                attr_request: bitmap4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for CB_GETATTR4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(CB_GETATTR4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for CB_GETATTR4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(CB_GETATTR4resok {
                obj_attributes: fattr4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for CB_ILLEGAL4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(CB_ILLEGAL4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for CB_RECALL4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(CB_RECALL4args {
                stateid: stateid4::try_from(&mut v)?,
                truncate: v.read_bool()?,
                fh: nfs_fh4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for CB_RECALL4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(CB_RECALL4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for CLOSE4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(CLOSE4args {
                seqid: seqid4::try_from(&mut v)?,
                open_stateid: stateid4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for CLOSE4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(stateid4::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for COMMIT4args {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(COMMIT4args {
                offset: offset4::try_from(&mut v)?,
                count: count4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for COMMIT4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(COMMIT4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for COMMIT4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(COMMIT4resok {
                writeverf: verifier4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for COMPOUND4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(COMPOUND4args {
                tag: utf8str_cs::try_from(&mut v)?,
                minorversion: v.read_u32()?,
                argarray: v.read_variable_array::<nfs_argop4<Bytes>>(None)?,
            })
        }
    }
    impl TryFrom<Bytes> for COMPOUND4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(COMPOUND4res {
                status: nfsstat4::try_from(&mut v)?,
                tag: utf8str_cs::try_from(&mut v)?,
                resarray: v.read_variable_array::<nfs_resop4<Bytes>>(None)?,
            })
        }
    }
    impl TryFrom<Bytes> for CREATE4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(CREATE4args {
                objtype: createtype4::try_from(&mut v)?,
                objname: component4::try_from(&mut v)?,
                createattrs: fattr4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for CREATE4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(CREATE4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for CREATE4resok {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(CREATE4resok {
                cinfo: change_info4::try_from(&mut v)?,
                attrset: bitmap4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for DELEGPURGE4args {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(DELEGPURGE4args {
                clientid: clientid4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for DELEGPURGE4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(DELEGPURGE4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for DELEGRETURN4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(DELEGRETURN4args {
                deleg_stateid: stateid4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for DELEGRETURN4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(DELEGRETURN4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for GETATTR4args {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(GETATTR4args {
                attr_request: bitmap4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for GETATTR4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(GETATTR4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for GETATTR4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(GETATTR4resok {
                obj_attributes: fattr4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for GETFH4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(GETFH4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for GETFH4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(GETFH4resok {
                object: nfs_fh4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for ILLEGAL4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(ILLEGAL4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for LINK4args {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(LINK4args {
                newname: component4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for LINK4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(LINK4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for LINK4resok {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(LINK4resok {
                cinfo: change_info4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for LOCK4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(LOCK4args {
                locktype: nfs_lock_type4::try_from(&mut v)?,
                reclaim: v.read_bool()?,
                offset: offset4::try_from(&mut v)?,
                length: length4::try_from(&mut v)?,
                locker: locker4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for LOCK4denied<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(LOCK4denied {
                offset: offset4::try_from(&mut v)?,
                length: length4::try_from(&mut v)?,
                locktype: nfs_lock_type4::try_from(&mut v)?,
                owner: lock_owner4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for LOCK4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(LOCK4resok::try_from(&mut v)?)
                }
                c if c == nfsstat4::NFS4ERR_DENIED as nfsstat4 => {
                    Self::NFS4ERR_DENIED(LOCK4denied::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for LOCK4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(LOCK4resok {
                lock_stateid: stateid4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for LOCKT4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(LOCKT4args {
                locktype: nfs_lock_type4::try_from(&mut v)?,
                offset: offset4::try_from(&mut v)?,
                length: length4::try_from(&mut v)?,
                owner: lock_owner4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for LOCKT4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4ERR_DENIED as nfsstat4 => {
                    Self::NFS4ERR_DENIED(LOCK4denied::try_from(&mut v)?)
                }
                c if c == nfsstat4::NFS4_OK as nfsstat4 => Self::NFS4_OK,
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for LOCKU4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(LOCKU4args {
                locktype: nfs_lock_type4::try_from(&mut v)?,
                seqid: seqid4::try_from(&mut v)?,
                lock_stateid: stateid4::try_from(&mut v)?,
                offset: offset4::try_from(&mut v)?,
                length: length4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for LOCKU4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(stateid4::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for LOOKUP4args {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let d = LOOKUP4args {
                objname: component4::try_from(&mut v)?,
            };

            debug!("LOOKUP {:?}", d);
            Ok(d)
        }
    }
    impl TryFrom<Bytes> for LOOKUP4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(LOOKUP4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for LOOKUPP4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(LOOKUPP4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for NVERIFY4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(NVERIFY4args {
                obj_attributes: fattr4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for NVERIFY4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(NVERIFY4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for OPEN4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(OPEN4args {
                seqid: seqid4::try_from(&mut v)?,
                share_access: v.read_u32()?,
                share_deny: v.read_u32()?,
                owner: open_owner4::try_from(&mut v)?,
                openhow: openflag4::try_from(&mut v)?,
                claim: open_claim4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for OPEN4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(OPEN4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for OPEN4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(OPEN4resok {
                stateid: stateid4::try_from(&mut v)?,
                cinfo: change_info4::try_from(&mut v)?,
                rflags: v.read_u32()?,
                attrset: bitmap4::try_from(&mut v)?,
                delegation: open_delegation4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for OPENATTR4args {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(OPENATTR4args {
                createdir: v.read_bool()?,
            })
        }
    }
    impl TryFrom<Bytes> for OPENATTR4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(OPENATTR4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for OPEN_CONFIRM4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(OPEN_CONFIRM4args {
                open_stateid: stateid4::try_from(&mut v)?,
                seqid: seqid4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for OPEN_CONFIRM4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(OPEN_CONFIRM4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for OPEN_CONFIRM4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(OPEN_CONFIRM4resok {
                open_stateid: stateid4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for OPEN_DOWNGRADE4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(OPEN_DOWNGRADE4args {
                open_stateid: stateid4::try_from(&mut v)?,
                seqid: seqid4::try_from(&mut v)?,
                share_access: v.read_u32()?,
                share_deny: v.read_u32()?,
            })
        }
    }
    impl TryFrom<Bytes> for OPEN_DOWNGRADE4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(OPEN_DOWNGRADE4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for OPEN_DOWNGRADE4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(OPEN_DOWNGRADE4resok {
                open_stateid: stateid4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for PUTFH4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(PUTFH4args {
                object: nfs_fh4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for PUTFH4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(PUTFH4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for PUTPUBFH4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(PUTPUBFH4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for PUTROOTFH4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(PUTROOTFH4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for READ4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(READ4args {
                stateid: stateid4::try_from(&mut v)?,
                offset: offset4::try_from(&mut v)?,
                count: count4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for READ4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(READ4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for READ4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(READ4resok {
                eof: v.read_bool()?,
                data: v.read_variable_bytes(None)?,
            })
        }
    }
    impl TryFrom<Bytes> for READDIR4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(READDIR4args {
                cookie: nfs_cookie4::try_from(&mut v)?,
                cookieverf: verifier4::try_from(&mut v)?,
                dircount: count4::try_from(&mut v)?,
                maxcount: count4::try_from(&mut v)?,
                attr_request: bitmap4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for READDIR4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(READDIR4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for READDIR4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(READDIR4resok {
                cookieverf: verifier4::try_from(&mut v)?,
                reply: dirlist4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for READLINK4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(READLINK4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for READLINK4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(READLINK4resok {
                link: linktext4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for RELEASE_LOCKOWNER4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(RELEASE_LOCKOWNER4args {
                lock_owner: lock_owner4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for RELEASE_LOCKOWNER4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(RELEASE_LOCKOWNER4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for REMOVE4args {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(REMOVE4args {
                target: component4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for REMOVE4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(REMOVE4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for REMOVE4resok {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(REMOVE4resok {
                cinfo: change_info4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for RENAME4args {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(RENAME4args {
                oldname: component4::try_from(&mut v)?,
                newname: component4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for RENAME4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(RENAME4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for RENAME4resok {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(RENAME4resok {
                source_cinfo: change_info4::try_from(&mut v)?,
                target_cinfo: change_info4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for RENEW4args {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(RENEW4args {
                clientid: clientid4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for RENEW4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(RENEW4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for RESTOREFH4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(RESTOREFH4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for SAVEFH4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(SAVEFH4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for SECINFO4args {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(SECINFO4args {
                name: component4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for SECINFO4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(SECINFO4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for SECINFO4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_array::<secinfo4<Bytes>>(None)?))
        }
    }
    impl TryFrom<Bytes> for SETATTR4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(SETATTR4args {
                stateid: stateid4::try_from(&mut v)?,
                obj_attributes: fattr4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for SETATTR4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(SETATTR4res {
                status: nfsstat4::try_from(&mut v)?,
                attrsset: bitmap4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for SETCLIENTID4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(SETCLIENTID4args {
                client: nfs_client_id4::try_from(&mut v)?,
                callback: cb_client4::try_from(&mut v)?,
                callback_ident: v.read_u32()?,
            })
        }
    }
    impl TryFrom<Bytes> for SETCLIENTID4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(SETCLIENTID4resok::try_from(&mut v)?)
                }
                c if c == nfsstat4::NFS4ERR_CLID_INUSE as nfsstat4 => {
                    Self::NFS4ERR_CLID_INUSE(clientaddr4::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for SETCLIENTID4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(SETCLIENTID4resok {
                clientid: clientid4::try_from(&mut v)?,
                setclientid_confirm: verifier4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for SETCLIENTID_CONFIRM4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(SETCLIENTID_CONFIRM4args {
                clientid: clientid4::try_from(&mut v)?,
                setclientid_confirm: verifier4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for SETCLIENTID_CONFIRM4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(SETCLIENTID_CONFIRM4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for VERIFY4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(VERIFY4args {
                obj_attributes: fattr4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for VERIFY4res {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(VERIFY4res {
                status: nfsstat4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for WRITE4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(WRITE4args {
                stateid: stateid4::try_from(&mut v)?,
                offset: offset4::try_from(&mut v)?,
                stable: stable_how4::try_from(&mut v)?,
                data: v.read_variable_bytes(None)?,
            })
        }
    }
    impl TryFrom<Bytes> for WRITE4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(WRITE4resok::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for WRITE4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(WRITE4resok {
                count: count4::try_from(&mut v)?,
                committed: stable_how4::try_from(&mut v)?,
                writeverf: verifier4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for aceflag4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for acemask4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for acetype4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for ascii_REQUIRED4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(utf8string::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for attrlist4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_bytes(None)?))
        }
    }
    impl TryFrom<Bytes> for bitmap4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_array::<bitmap_inner>(None)?))
        }
    }
    impl TryFrom<Bytes> for bitmap_inner {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for cb_client4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(cb_client4 {
                cb_program: v.read_u32()?,
                cb_location: clientaddr4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for change_info4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(change_info4 {
                atomic: v.read_bool()?,
                before: changeid4::try_from(&mut v)?,
                after: changeid4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for changeid4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for clientaddr4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(clientaddr4 {
                r_netid: v.read_string(None)?,
                r_addr: v.read_string(None)?,
            })
        }
    }
    impl TryFrom<Bytes> for clientid4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for component4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(utf8str_cs::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for count4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for createhow4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let mode = createmode4::try_from(&mut v)?;
            Ok(match mode {
                c if c == createmode4::UNCHECKED4 as createmode4 => {
                    Self::UNCHECKED4(fattr4::try_from(&mut v)?)
                }
                c if c == createmode4::GUARDED4 as createmode4 => {
                    Self::GUARDED4(fattr4::try_from(&mut v)?)
                }
                c if c == createmode4::EXCLUSIVE4 as createmode4 => {
                    Self::EXCLUSIVE4(verifier4::try_from(&mut v)?)
                }
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for createmode4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                0 => Self::UNCHECKED4,
                1 => Self::GUARDED4,
                2 => Self::EXCLUSIVE4,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for createtype4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let type_v = nfs_ftype4::try_from(&mut v)?;
            Ok(match type_v {
                c if c == nfs_ftype4::NF4LNK as nfs_ftype4 => {
                    Self::NF4LNK(linktext4::try_from(&mut v)?)
                }
                c if c == nfs_ftype4::NF4BLK as nfs_ftype4 => {
                    Self::NF4BLK(specdata4::try_from(&mut v)?)
                }
                c if c == nfs_ftype4::NF4CHR as nfs_ftype4 => {
                    Self::NF4CHR(specdata4::try_from(&mut v)?)
                }
                c if c == nfs_ftype4::NF4SOCK as nfs_ftype4 => Self::NF4SOCK,
                c if c == nfs_ftype4::NF4FIFO as nfs_ftype4 => Self::NF4FIFO,
                c if c == nfs_ftype4::NF4DIR as nfs_ftype4 => Self::NF4DIR,
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for dirlist4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(dirlist4 {
                entries: {
                    match v.read_u32()? {
                        0 => None,
                        1 => Some(Box::new(entry4::try_from(&mut v)?)),
                        d => return Err(Error::UnknownOptionVariant(d)),
                    }
                },
                eof: v.read_bool()?,
            })
        }
    }
    impl TryFrom<Bytes> for entry4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(entry4 {
                cookie: nfs_cookie4::try_from(&mut v)?,
                name: component4::try_from(&mut v)?,
                attrs: fattr4::try_from(&mut v)?,
                nextentry: {
                    match v.read_u32()? {
                        0 => None,
                        1 => Some(Box::new(entry4::try_from(&mut v)?)),
                        d => return Err(Error::UnknownOptionVariant(d)),
                    }
                },
            })
        }
    }
    impl TryFrom<Bytes> for exist_lock_owner4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(exist_lock_owner4 {
                lock_stateid: stateid4::try_from(&mut v)?,
                lock_seqid: seqid4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for fattr4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(fattr4 {
                attrmask: bitmap4::try_from(&mut v)?,
                attr_vals: attrlist4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for fattr4_acl<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_array::<nfsace4<Bytes>>(None)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_aclsupport {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_archive {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_cansettime {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_case_insensitive {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_case_preserving {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_change {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(changeid4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_chown_restricted {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_fh_expire_type {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_filehandle<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfs_fh4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_fileid {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_files_avail {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_files_free {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_files_total {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_fs_locations {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(fs_locations4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_fsid {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(fsid4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_hidden {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_homogeneous {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_lease_time {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfs_lease4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_link_support {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_maxfilesize {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_maxlink {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_maxname {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_maxread {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_maxwrite {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_mimetype<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(ascii_REQUIRED4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_mode {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(mode4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_mounted_on_fileid {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_named_attr {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_no_trunc {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_numlinks {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_owner<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(utf8str_mixed::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_owner_group<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(utf8str_mixed::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_quota_avail_hard {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_quota_avail_soft {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_quota_used {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_rawdev {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(specdata4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_rdattr_error {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfsstat4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_size {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_space_avail {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_space_free {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_space_total {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_space_used {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_supported_attrs {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(bitmap4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_symlink_support {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_system {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<Bytes> for fattr4_time_access {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfstime4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_time_access_set {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(settime4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_time_backup {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfstime4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_time_create {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfstime4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_time_delta {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfstime4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_time_metadata {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfstime4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_time_modify {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfstime4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_time_modify_set {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(settime4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_type {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfs_ftype4::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for fattr4_unique_handles {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<Bytes> for fs_location4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(fs_location4 {
                server: v.read_variable_array::<utf8str_cis>(None)?,
                rootpath: pathname4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for fs_locations4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(fs_locations4 {
                fs_root: pathname4::try_from(&mut v)?,
                locations: v.read_variable_array::<fs_location4>(None)?,
            })
        }
    }
    impl TryFrom<Bytes> for fsid4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(fsid4 {
                major: v.read_u64()?,
                minor: v.read_u64()?,
            })
        }
    }
    impl TryFrom<Bytes> for length4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for limit_by4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                1 => Self::NFS_LIMIT_SIZE,
                2 => Self::NFS_LIMIT_BLOCKS,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for linktext4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_bytes(None)?))
        }
    }
    impl TryFrom<Bytes> for lock_owner4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(lock_owner4 {
                clientid: clientid4::try_from(&mut v)?,
                owner: v.read_variable_bytes(Some(1024))?,
            })
        }
    }
    impl TryFrom<Bytes> for locker4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let new_lock_owner = v.read_bool()?;
            Ok(match new_lock_owner {
                true => Self::TRUE(open_to_lock_owner4::try_from(&mut v)?),
                false => Self::FALSE(exist_lock_owner4::try_from(&mut v)?),
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for mode4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for nfs_argop4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let argop = nfs_opnum4::try_from(&mut v)?;
            Ok(match argop {
                c if c == nfs_opnum4::OP_ACCESS as nfs_opnum4 => {
                    Self::OP_ACCESS(ACCESS4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_CLOSE as nfs_opnum4 => {
                    Self::OP_CLOSE(CLOSE4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_COMMIT as nfs_opnum4 => {
                    Self::OP_COMMIT(COMMIT4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_CREATE as nfs_opnum4 => {
                    Self::OP_CREATE(CREATE4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_DELEGPURGE as nfs_opnum4 => {
                    Self::OP_DELEGPURGE(DELEGPURGE4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_DELEGRETURN as nfs_opnum4 => {
                    Self::OP_DELEGRETURN(DELEGRETURN4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_GETATTR as nfs_opnum4 => {
                    Self::OP_GETATTR(GETATTR4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_LINK as nfs_opnum4 => {
                    Self::OP_LINK(LINK4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_LOCK as nfs_opnum4 => {
                    Self::OP_LOCK(LOCK4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_LOCKT as nfs_opnum4 => {
                    Self::OP_LOCKT(LOCKT4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_LOCKU as nfs_opnum4 => {
                    Self::OP_LOCKU(LOCKU4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_LOOKUP as nfs_opnum4 => {
                    Self::OP_LOOKUP(LOOKUP4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_NVERIFY as nfs_opnum4 => {
                    Self::OP_NVERIFY(NVERIFY4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_OPEN as nfs_opnum4 => {
                    Self::OP_OPEN(OPEN4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_OPENATTR as nfs_opnum4 => {
                    Self::OP_OPENATTR(OPENATTR4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_OPEN_CONFIRM as nfs_opnum4 => {
                    Self::OP_OPEN_CONFIRM(OPEN_CONFIRM4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_OPEN_DOWNGRADE as nfs_opnum4 => {
                    Self::OP_OPEN_DOWNGRADE(OPEN_DOWNGRADE4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_PUTFH as nfs_opnum4 => {
                    Self::OP_PUTFH(PUTFH4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_READ as nfs_opnum4 => {
                    Self::OP_READ(READ4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_READDIR as nfs_opnum4 => {
                    Self::OP_READDIR(READDIR4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_REMOVE as nfs_opnum4 => {
                    Self::OP_REMOVE(REMOVE4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_RENAME as nfs_opnum4 => {
                    Self::OP_RENAME(RENAME4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_RENEW as nfs_opnum4 => {
                    Self::OP_RENEW(RENEW4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_SECINFO as nfs_opnum4 => {
                    Self::OP_SECINFO(SECINFO4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_SETATTR as nfs_opnum4 => {
                    Self::OP_SETATTR(SETATTR4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_SETCLIENTID as nfs_opnum4 => {
                    Self::OP_SETCLIENTID(SETCLIENTID4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_SETCLIENTID_CONFIRM as nfs_opnum4 => {
                    Self::OP_SETCLIENTID_CONFIRM(SETCLIENTID_CONFIRM4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_VERIFY as nfs_opnum4 => {
                    Self::OP_VERIFY(VERIFY4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_WRITE as nfs_opnum4 => {
                    Self::OP_WRITE(WRITE4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_RELEASE_LOCKOWNER as nfs_opnum4 => {
                    Self::OP_RELEASE_LOCKOWNER(RELEASE_LOCKOWNER4args::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_GETFH as nfs_opnum4 => Self::OP_GETFH,
                c if c == nfs_opnum4::OP_LOOKUPP as nfs_opnum4 => Self::OP_LOOKUPP,
                c if c == nfs_opnum4::OP_PUTPUBFH as nfs_opnum4 => Self::OP_PUTPUBFH,
                c if c == nfs_opnum4::OP_PUTROOTFH as nfs_opnum4 => Self::OP_PUTROOTFH,
                c if c == nfs_opnum4::OP_READLINK as nfs_opnum4 => Self::OP_READLINK,
                c if c == nfs_opnum4::OP_RESTOREFH as nfs_opnum4 => Self::OP_RESTOREFH,
                c if c == nfs_opnum4::OP_SAVEFH as nfs_opnum4 => Self::OP_SAVEFH,
                c if c == nfs_opnum4::OP_ILLEGAL as nfs_opnum4 => Self::OP_ILLEGAL,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for nfs_cb_argop4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let argop = v.read_u32()?;
            Ok(match argop {
                c if c == nfs_cb_opnum4::OP_CB_GETATTR as u32 => {
                    Self::OP_CB_GETATTR(CB_GETATTR4args::try_from(&mut v)?)
                }
                c if c == nfs_cb_opnum4::OP_CB_RECALL as u32 => {
                    Self::OP_CB_RECALL(CB_RECALL4args::try_from(&mut v)?)
                }
                c if c == nfs_cb_opnum4::OP_CB_ILLEGAL as u32 => Self::OP_CB_ILLEGAL,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for nfs_cb_opnum4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                3 => Self::OP_CB_GETATTR,
                4 => Self::OP_CB_RECALL,
                10044 => Self::OP_CB_ILLEGAL,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for nfs_cb_resop4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let resop = v.read_u32()?;
            Ok(match resop {
                c if c == nfs_cb_opnum4::OP_CB_GETATTR as u32 => {
                    Self::OP_CB_GETATTR(CB_GETATTR4res::try_from(&mut v)?)
                }
                c if c == nfs_cb_opnum4::OP_CB_RECALL as u32 => {
                    Self::OP_CB_RECALL(CB_RECALL4res::try_from(&mut v)?)
                }
                c if c == nfs_cb_opnum4::OP_CB_ILLEGAL as u32 => {
                    Self::OP_CB_ILLEGAL(CB_ILLEGAL4res::try_from(&mut v)?)
                }
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for nfs_client_id4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(nfs_client_id4 {
                verifier: verifier4::try_from(&mut v)?,
                id: v.read_variable_bytes(Some(1024))?,
            })
        }
    }
    impl TryFrom<Bytes> for nfs_cookie4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for nfs_fh4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_bytes(None)?))
        }
    }
    impl TryFrom<Bytes> for nfs_ftype4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                1 => Self::NF4REG,
                2 => Self::NF4DIR,
                3 => Self::NF4BLK,
                4 => Self::NF4CHR,
                5 => Self::NF4LNK,
                6 => Self::NF4SOCK,
                7 => Self::NF4FIFO,
                8 => Self::NF4ATTRDIR,
                9 => Self::NF4NAMEDATTR,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for nfs_lease4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for nfs_lock_type4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                1 => Self::READ_LT,
                2 => Self::WRITE_LT,
                3 => Self::READW_LT,
                4 => Self::WRITEW_LT,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for nfs_lockid4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for nfs_modified_limit4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(nfs_modified_limit4 {
                num_blocks: v.read_u32()?,
                bytes_per_block: v.read_u32()?,
            })
        }
    }
    impl TryFrom<Bytes> for nfs_opnum4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                3 => Self::OP_ACCESS,
                4 => Self::OP_CLOSE,
                5 => Self::OP_COMMIT,
                6 => Self::OP_CREATE,
                7 => Self::OP_DELEGPURGE,
                8 => Self::OP_DELEGRETURN,
                9 => Self::OP_GETATTR,
                10 => Self::OP_GETFH,
                11 => Self::OP_LINK,
                12 => Self::OP_LOCK,
                13 => Self::OP_LOCKT,
                14 => Self::OP_LOCKU,
                15 => Self::OP_LOOKUP,
                16 => Self::OP_LOOKUPP,
                17 => Self::OP_NVERIFY,
                18 => Self::OP_OPEN,
                19 => Self::OP_OPENATTR,
                20 => Self::OP_OPEN_CONFIRM,
                21 => Self::OP_OPEN_DOWNGRADE,
                22 => Self::OP_PUTFH,
                23 => Self::OP_PUTPUBFH,
                24 => Self::OP_PUTROOTFH,
                25 => Self::OP_READ,
                26 => Self::OP_READDIR,
                27 => Self::OP_READLINK,
                28 => Self::OP_REMOVE,
                29 => Self::OP_RENAME,
                30 => Self::OP_RENEW,
                31 => Self::OP_RESTOREFH,
                32 => Self::OP_SAVEFH,
                33 => Self::OP_SECINFO,
                34 => Self::OP_SETATTR,
                35 => Self::OP_SETCLIENTID,
                36 => Self::OP_SETCLIENTID_CONFIRM,
                37 => Self::OP_VERIFY,
                38 => Self::OP_WRITE,
                39 => Self::OP_RELEASE_LOCKOWNER,
                10044 => Self::OP_ILLEGAL,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for nfs_resop4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let resop = nfs_opnum4::try_from(&mut v)?;
            Ok(match resop {
                c if c == nfs_opnum4::OP_ACCESS as nfs_opnum4 => {
                    Self::OP_ACCESS(ACCESS4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_CLOSE as nfs_opnum4 => {
                    Self::OP_CLOSE(CLOSE4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_COMMIT as nfs_opnum4 => {
                    Self::OP_COMMIT(COMMIT4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_CREATE as nfs_opnum4 => {
                    Self::OP_CREATE(CREATE4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_DELEGPURGE as nfs_opnum4 => {
                    Self::OP_DELEGPURGE(DELEGPURGE4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_DELEGRETURN as nfs_opnum4 => {
                    Self::OP_DELEGRETURN(DELEGRETURN4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_GETATTR as nfs_opnum4 => {
                    Self::OP_GETATTR(GETATTR4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_GETFH as nfs_opnum4 => {
                    Self::OP_GETFH(GETFH4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_LINK as nfs_opnum4 => {
                    Self::OP_LINK(LINK4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_LOCK as nfs_opnum4 => {
                    Self::OP_LOCK(LOCK4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_LOCKT as nfs_opnum4 => {
                    Self::OP_LOCKT(LOCKT4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_LOCKU as nfs_opnum4 => {
                    Self::OP_LOCKU(LOCKU4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_LOOKUP as nfs_opnum4 => {
                    Self::OP_LOOKUP(LOOKUP4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_LOOKUPP as nfs_opnum4 => {
                    Self::OP_LOOKUPP(LOOKUPP4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_NVERIFY as nfs_opnum4 => {
                    Self::OP_NVERIFY(NVERIFY4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_OPEN as nfs_opnum4 => {
                    Self::OP_OPEN(OPEN4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_OPENATTR as nfs_opnum4 => {
                    Self::OP_OPENATTR(OPENATTR4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_OPEN_CONFIRM as nfs_opnum4 => {
                    Self::OP_OPEN_CONFIRM(OPEN_CONFIRM4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_OPEN_DOWNGRADE as nfs_opnum4 => {
                    Self::OP_OPEN_DOWNGRADE(OPEN_DOWNGRADE4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_PUTFH as nfs_opnum4 => {
                    Self::OP_PUTFH(PUTFH4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_PUTPUBFH as nfs_opnum4 => {
                    Self::OP_PUTPUBFH(PUTPUBFH4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_PUTROOTFH as nfs_opnum4 => {
                    Self::OP_PUTROOTFH(PUTROOTFH4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_READ as nfs_opnum4 => {
                    Self::OP_READ(READ4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_READDIR as nfs_opnum4 => {
                    Self::OP_READDIR(READDIR4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_READLINK as nfs_opnum4 => {
                    Self::OP_READLINK(READLINK4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_REMOVE as nfs_opnum4 => {
                    Self::OP_REMOVE(REMOVE4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_RENAME as nfs_opnum4 => {
                    Self::OP_RENAME(RENAME4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_RENEW as nfs_opnum4 => {
                    Self::OP_RENEW(RENEW4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_RESTOREFH as nfs_opnum4 => {
                    Self::OP_RESTOREFH(RESTOREFH4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_SAVEFH as nfs_opnum4 => {
                    Self::OP_SAVEFH(SAVEFH4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_SECINFO as nfs_opnum4 => {
                    Self::OP_SECINFO(SECINFO4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_SETATTR as nfs_opnum4 => {
                    Self::OP_SETATTR(SETATTR4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_SETCLIENTID as nfs_opnum4 => {
                    Self::OP_SETCLIENTID(SETCLIENTID4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_SETCLIENTID_CONFIRM as nfs_opnum4 => {
                    Self::OP_SETCLIENTID_CONFIRM(SETCLIENTID_CONFIRM4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_VERIFY as nfs_opnum4 => {
                    Self::OP_VERIFY(VERIFY4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_WRITE as nfs_opnum4 => {
                    Self::OP_WRITE(WRITE4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_RELEASE_LOCKOWNER as nfs_opnum4 => {
                    Self::OP_RELEASE_LOCKOWNER(RELEASE_LOCKOWNER4res::try_from(&mut v)?)
                }
                c if c == nfs_opnum4::OP_ILLEGAL as nfs_opnum4 => {
                    Self::OP_ILLEGAL(ILLEGAL4res::try_from(&mut v)?)
                }
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for nfs_space_limit4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let limitby = limit_by4::try_from(&mut v)?;
            Ok(match limitby {
                c if c == limit_by4::NFS_LIMIT_SIZE as limit_by4 => {
                    Self::NFS_LIMIT_SIZE(v.read_u64()?)
                }
                c if c == limit_by4::NFS_LIMIT_BLOCKS as limit_by4 => {
                    Self::NFS_LIMIT_BLOCKS(nfs_modified_limit4::try_from(&mut v)?)
                }
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for nfsace4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(nfsace4 {
                type_v: acetype4::try_from(&mut v)?,
                flag: aceflag4::try_from(&mut v)?,
                access_mask: acemask4::try_from(&mut v)?,
                who: utf8str_mixed::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for nfsstat4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                0 => Self::NFS4_OK,
                1 => Self::NFS4ERR_PERM,
                2 => Self::NFS4ERR_NOENT,
                5 => Self::NFS4ERR_IO,
                6 => Self::NFS4ERR_NXIO,
                13 => Self::NFS4ERR_ACCESS,
                17 => Self::NFS4ERR_EXIST,
                18 => Self::NFS4ERR_XDEV,
                20 => Self::NFS4ERR_NOTDIR,
                21 => Self::NFS4ERR_ISDIR,
                22 => Self::NFS4ERR_INVAL,
                27 => Self::NFS4ERR_FBIG,
                28 => Self::NFS4ERR_NOSPC,
                30 => Self::NFS4ERR_ROFS,
                31 => Self::NFS4ERR_MLINK,
                63 => Self::NFS4ERR_NAMETOOLONG,
                66 => Self::NFS4ERR_NOTEMPTY,
                69 => Self::NFS4ERR_DQUOT,
                70 => Self::NFS4ERR_STALE,
                10001 => Self::NFS4ERR_BADHANDLE,
                10003 => Self::NFS4ERR_BAD_COOKIE,
                10004 => Self::NFS4ERR_NOTSUPP,
                10005 => Self::NFS4ERR_TOOSMALL,
                10006 => Self::NFS4ERR_SERVERFAULT,
                10007 => Self::NFS4ERR_BADTYPE,
                10008 => Self::NFS4ERR_DELAY,
                10009 => Self::NFS4ERR_SAME,
                10010 => Self::NFS4ERR_DENIED,
                10011 => Self::NFS4ERR_EXPIRED,
                10012 => Self::NFS4ERR_LOCKED,
                10013 => Self::NFS4ERR_GRACE,
                10014 => Self::NFS4ERR_FHEXPIRED,
                10015 => Self::NFS4ERR_SHARE_DENIED,
                10016 => Self::NFS4ERR_WRONGSEC,
                10017 => Self::NFS4ERR_CLID_INUSE,
                10018 => Self::NFS4ERR_RESOURCE,
                10019 => Self::NFS4ERR_MOVED,
                10020 => Self::NFS4ERR_NOFILEHANDLE,
                10021 => Self::NFS4ERR_MINOR_VERS_MISMATCH,
                10022 => Self::NFS4ERR_STALE_CLIENTID,
                10023 => Self::NFS4ERR_STALE_STATEID,
                10024 => Self::NFS4ERR_OLD_STATEID,
                10025 => Self::NFS4ERR_BAD_STATEID,
                10026 => Self::NFS4ERR_BAD_SEQID,
                10027 => Self::NFS4ERR_NOT_SAME,
                10028 => Self::NFS4ERR_LOCK_RANGE,
                10029 => Self::NFS4ERR_SYMLINK,
                10030 => Self::NFS4ERR_RESTOREFH,
                10031 => Self::NFS4ERR_LEASE_MOVED,
                10032 => Self::NFS4ERR_ATTRNOTSUPP,
                10033 => Self::NFS4ERR_NO_GRACE,
                10034 => Self::NFS4ERR_RECLAIM_BAD,
                10035 => Self::NFS4ERR_RECLAIM_CONFLICT,
                10036 => Self::NFS4ERR_BADXDR,
                10037 => Self::NFS4ERR_LOCKS_HELD,
                10038 => Self::NFS4ERR_OPENMODE,
                10039 => Self::NFS4ERR_BADOWNER,
                10040 => Self::NFS4ERR_BADCHAR,
                10041 => Self::NFS4ERR_BADNAME,
                10042 => Self::NFS4ERR_BAD_RANGE,
                10043 => Self::NFS4ERR_LOCK_NOTSUPP,
                10044 => Self::NFS4ERR_OP_ILLEGAL,
                10045 => Self::NFS4ERR_DEADLOCK,
                10046 => Self::NFS4ERR_FILE_OPEN,
                10047 => Self::NFS4ERR_ADMIN_REVOKED,
                10048 => Self::NFS4ERR_CB_PATH_DOWN,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for nfstime4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(nfstime4 {
                seconds: v.read_i64()?,
                nseconds: v.read_u32()?,
            })
        }
    }
    impl TryFrom<Bytes> for offset4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<Bytes> for open_claim4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let claim = open_claim_type4::try_from(&mut v)?;
            Ok(match claim {
                c if c == open_claim_type4::CLAIM_NULL as open_claim_type4 => {
                    Self::CLAIM_NULL(component4::try_from(&mut v)?)
                }
                c if c == open_claim_type4::CLAIM_PREVIOUS as open_claim_type4 => {
                    Self::CLAIM_PREVIOUS(open_delegation_type4::try_from(&mut v)?)
                }
                c if c == open_claim_type4::CLAIM_DELEGATE_CUR as open_claim_type4 => {
                    Self::CLAIM_DELEGATE_CUR(open_claim_delegate_cur4::try_from(&mut v)?)
                }
                c if c == open_claim_type4::CLAIM_DELEGATE_PREV as open_claim_type4 => {
                    Self::CLAIM_DELEGATE_PREV(component4::try_from(&mut v)?)
                }
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for open_claim_delegate_cur4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(open_claim_delegate_cur4 {
                delegate_stateid: stateid4::try_from(&mut v)?,
                file: component4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for open_claim_type4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                0 => Self::CLAIM_NULL,
                1 => Self::CLAIM_PREVIOUS,
                2 => Self::CLAIM_DELEGATE_CUR,
                3 => Self::CLAIM_DELEGATE_PREV,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for open_delegation4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let delegation_type = open_delegation_type4::try_from(&mut v)?;
            Ok(match delegation_type {
                c if c == open_delegation_type4::OPEN_DELEGATE_READ as open_delegation_type4 => {
                    Self::OPEN_DELEGATE_READ(open_read_delegation4::try_from(&mut v)?)
                }
                c if c == open_delegation_type4::OPEN_DELEGATE_WRITE as open_delegation_type4 => {
                    Self::OPEN_DELEGATE_WRITE(open_write_delegation4::try_from(&mut v)?)
                }
                c if c == open_delegation_type4::OPEN_DELEGATE_NONE as open_delegation_type4 => {
                    Self::OPEN_DELEGATE_NONE
                }
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for open_delegation_type4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                0 => Self::OPEN_DELEGATE_NONE,
                1 => Self::OPEN_DELEGATE_READ,
                2 => Self::OPEN_DELEGATE_WRITE,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for open_owner4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(open_owner4 {
                clientid: clientid4::try_from(&mut v)?,
                owner: v.read_variable_bytes(Some(1024))?,
            })
        }
    }
    impl TryFrom<Bytes> for open_read_delegation4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(open_read_delegation4 {
                stateid: stateid4::try_from(&mut v)?,
                recall: v.read_bool()?,
                permissions: nfsace4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for open_to_lock_owner4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(open_to_lock_owner4 {
                open_seqid: seqid4::try_from(&mut v)?,
                open_stateid: stateid4::try_from(&mut v)?,
                lock_seqid: seqid4::try_from(&mut v)?,
                lock_owner: lock_owner4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for open_write_delegation4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(open_write_delegation4 {
                stateid: stateid4::try_from(&mut v)?,
                recall: v.read_bool()?,
                space_limit: nfs_space_limit4::try_from(&mut v)?,
                permissions: nfsace4::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for openflag4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let opentype = opentype4::try_from(&mut v)?;
            Ok(match opentype {
                c if c == opentype4::OPEN4_CREATE as opentype4 => {
                    Self::OPEN4_CREATE(createhow4::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for opentype4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                0 => Self::OPEN4_NOCREATE,
                1 => Self::OPEN4_CREATE,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for pathname4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_array::<component4>(None)?))
        }
    }
    impl TryFrom<Bytes> for qop4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for rpc_gss_svc_t {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                1 => Self::RPC_GSS_SVC_NONE,
                2 => Self::RPC_GSS_SVC_INTEGRITY,
                3 => Self::RPC_GSS_SVC_PRIVACY,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for rpcsec_gss_info<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(rpcsec_gss_info {
                oid: sec_oid4::try_from(&mut v)?,
                qop: qop4::try_from(&mut v)?,
                service: rpc_gss_svc_t::try_from(&mut v)?,
            })
        }
    }
    impl TryFrom<Bytes> for sec_oid4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_bytes(None)?))
        }
    }
    impl TryFrom<Bytes> for secinfo4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let flavor = v.read_u32()?;
            Ok(match flavor {
                RPCSEC_GSS => Self::RPCSEC_GSS(rpcsec_gss_info::try_from(&mut v)?),
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for seqid4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<Bytes> for settime4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            let set_it = time_how4::try_from(&mut v)?;
            Ok(match set_it {
                c if c == time_how4::SET_TO_CLIENT_TIME4 as time_how4 => {
                    Self::SET_TO_CLIENT_TIME4(nfstime4::try_from(&mut v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<Bytes> for specdata4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(specdata4 {
                specdata1: v.read_u32()?,
                specdata2: v.read_u32()?,
            })
        }
    }
    impl TryFrom<Bytes> for stable_how4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                0 => Self::UNSTABLE4,
                1 => Self::DATA_SYNC4,
                2 => Self::FILE_SYNC4,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for stateid4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(stateid4 {
                seqid: v.read_u32()?,
                other: v.read_bytes(12)?,
            })
        }
    }
    impl TryFrom<Bytes> for time_how4 {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                0 => Self::SET_TO_SERVER_TIME4,
                1 => Self::SET_TO_CLIENT_TIME4,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<Bytes> for utf8str_cis {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_string(None)?))
        }
    }
    impl TryFrom<Bytes> for utf8str_cs {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_string(None)?))
        }
    }
    impl TryFrom<Bytes> for utf8str_mixed<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(utf8string::try_from(&mut v)?))
        }
    }
    impl TryFrom<Bytes> for utf8string<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_bytes(None)?))
        }
    }
    impl TryFrom<Bytes> for verifier4<Bytes> {
        type Error = Error;

        fn try_from(mut v: Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bytes(8)?))
        }
    }
    impl TryFrom<&mut Bytes> for ACCESS4args {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(ACCESS4args {
                access: v.read_u32()?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for ACCESS4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(ACCESS4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for ACCESS4resok {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(ACCESS4resok {
                supported: v.read_u32()?,
                access: v.read_u32()?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for CB_COMPOUND4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(CB_COMPOUND4args {
                tag: utf8str_cs::try_from(&mut *v)?,
                minorversion: v.read_u32()?,
                callback_ident: v.read_u32()?,
                argarray: v.read_variable_array::<nfs_cb_argop4<Bytes>>(None)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for CB_COMPOUND4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(CB_COMPOUND4res {
                status: nfsstat4::try_from(&mut *v)?,
                tag: utf8str_cs::try_from(&mut *v)?,
                resarray: v.read_variable_array::<nfs_cb_resop4<Bytes>>(None)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for CB_GETATTR4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(CB_GETATTR4args {
                fh: nfs_fh4::try_from(&mut *v)?,
                attr_request: bitmap4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for CB_GETATTR4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(CB_GETATTR4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for CB_GETATTR4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(CB_GETATTR4resok {
                obj_attributes: fattr4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for CB_ILLEGAL4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(CB_ILLEGAL4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for CB_RECALL4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(CB_RECALL4args {
                stateid: stateid4::try_from(&mut *v)?,
                truncate: v.read_bool()?,
                fh: nfs_fh4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for CB_RECALL4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(CB_RECALL4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for CLOSE4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(CLOSE4args {
                seqid: seqid4::try_from(&mut *v)?,
                open_stateid: stateid4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for CLOSE4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(stateid4::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for COMMIT4args {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(COMMIT4args {
                offset: offset4::try_from(&mut *v)?,
                count: count4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for COMMIT4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(COMMIT4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for COMMIT4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(COMMIT4resok {
                writeverf: verifier4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for COMPOUND4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(COMPOUND4args {
                tag: utf8str_cs::try_from(&mut *v)?,
                minorversion: v.read_u32()?,
                argarray: v.read_variable_array::<nfs_argop4<Bytes>>(None)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for COMPOUND4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(COMPOUND4res {
                status: nfsstat4::try_from(&mut *v)?,
                tag: utf8str_cs::try_from(&mut *v)?,
                resarray: v.read_variable_array::<nfs_resop4<Bytes>>(None)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for CREATE4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(CREATE4args {
                objtype: createtype4::try_from(&mut *v)?,
                objname: component4::try_from(&mut *v)?,
                createattrs: fattr4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for CREATE4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(CREATE4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for CREATE4resok {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(CREATE4resok {
                cinfo: change_info4::try_from(&mut *v)?,
                attrset: bitmap4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for DELEGPURGE4args {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(DELEGPURGE4args {
                clientid: clientid4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for DELEGPURGE4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(DELEGPURGE4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for DELEGRETURN4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(DELEGRETURN4args {
                deleg_stateid: stateid4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for DELEGRETURN4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(DELEGRETURN4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for GETATTR4args {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(GETATTR4args {
                attr_request: bitmap4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for GETATTR4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(GETATTR4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for GETATTR4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(GETATTR4resok {
                obj_attributes: fattr4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for GETFH4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(GETFH4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for GETFH4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(GETFH4resok {
                object: nfs_fh4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for ILLEGAL4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(ILLEGAL4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LINK4args {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(LINK4args {
                newname: component4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LINK4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(LINK4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LINK4resok {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(LINK4resok {
                cinfo: change_info4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LOCK4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(LOCK4args {
                locktype: nfs_lock_type4::try_from(&mut *v)?,
                reclaim: v.read_bool()?,
                offset: offset4::try_from(&mut *v)?,
                length: length4::try_from(&mut *v)?,
                locker: locker4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LOCK4denied<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(LOCK4denied {
                offset: offset4::try_from(&mut *v)?,
                length: length4::try_from(&mut *v)?,
                locktype: nfs_lock_type4::try_from(&mut *v)?,
                owner: lock_owner4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LOCK4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(LOCK4resok::try_from(&mut *v)?)
                }
                c if c == nfsstat4::NFS4ERR_DENIED as nfsstat4 => {
                    Self::NFS4ERR_DENIED(LOCK4denied::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LOCK4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(LOCK4resok {
                lock_stateid: stateid4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LOCKT4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(LOCKT4args {
                locktype: nfs_lock_type4::try_from(&mut *v)?,
                offset: offset4::try_from(&mut *v)?,
                length: length4::try_from(&mut *v)?,
                owner: lock_owner4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LOCKT4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4ERR_DENIED as nfsstat4 => {
                    Self::NFS4ERR_DENIED(LOCK4denied::try_from(&mut *v)?)
                }
                c if c == nfsstat4::NFS4_OK as nfsstat4 => Self::NFS4_OK,
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LOCKU4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(LOCKU4args {
                locktype: nfs_lock_type4::try_from(&mut *v)?,
                seqid: seqid4::try_from(&mut *v)?,
                lock_stateid: stateid4::try_from(&mut *v)?,
                offset: offset4::try_from(&mut *v)?,
                length: length4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LOCKU4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(stateid4::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LOOKUP4args {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(LOOKUP4args {
                objname: component4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LOOKUP4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(LOOKUP4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for LOOKUPP4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(LOOKUPP4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for NVERIFY4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(NVERIFY4args {
                obj_attributes: fattr4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for NVERIFY4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(NVERIFY4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for OPEN4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(OPEN4args {
                seqid: seqid4::try_from(&mut *v)?,
                share_access: v.read_u32()?,
                share_deny: v.read_u32()?,
                owner: open_owner4::try_from(&mut *v)?,
                openhow: openflag4::try_from(&mut *v)?,
                claim: open_claim4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for OPEN4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(OPEN4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for OPEN4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(OPEN4resok {
                stateid: stateid4::try_from(&mut *v)?,
                cinfo: change_info4::try_from(&mut *v)?,
                rflags: v.read_u32()?,
                attrset: bitmap4::try_from(&mut *v)?,
                delegation: open_delegation4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for OPENATTR4args {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(OPENATTR4args {
                createdir: v.read_bool()?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for OPENATTR4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(OPENATTR4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for OPEN_CONFIRM4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(OPEN_CONFIRM4args {
                open_stateid: stateid4::try_from(&mut *v)?,
                seqid: seqid4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for OPEN_CONFIRM4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(OPEN_CONFIRM4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for OPEN_CONFIRM4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(OPEN_CONFIRM4resok {
                open_stateid: stateid4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for OPEN_DOWNGRADE4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(OPEN_DOWNGRADE4args {
                open_stateid: stateid4::try_from(&mut *v)?,
                seqid: seqid4::try_from(&mut *v)?,
                share_access: v.read_u32()?,
                share_deny: v.read_u32()?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for OPEN_DOWNGRADE4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(OPEN_DOWNGRADE4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for OPEN_DOWNGRADE4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(OPEN_DOWNGRADE4resok {
                open_stateid: stateid4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for PUTFH4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(PUTFH4args {
                object: nfs_fh4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for PUTFH4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(PUTFH4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for PUTPUBFH4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(PUTPUBFH4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for PUTROOTFH4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(PUTROOTFH4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for READ4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(READ4args {
                stateid: stateid4::try_from(&mut *v)?,
                offset: offset4::try_from(&mut *v)?,
                count: count4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for READ4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(READ4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for READ4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(READ4resok {
                eof: v.read_bool()?,
                data: v.read_variable_bytes(None)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for READDIR4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(READDIR4args {
                cookie: nfs_cookie4::try_from(&mut *v)?,
                cookieverf: verifier4::try_from(&mut *v)?,
                dircount: count4::try_from(&mut *v)?,
                maxcount: count4::try_from(&mut *v)?,
                attr_request: bitmap4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for READDIR4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(READDIR4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for READDIR4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(READDIR4resok {
                cookieverf: verifier4::try_from(&mut *v)?,
                reply: dirlist4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for READLINK4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(READLINK4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for READLINK4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(READLINK4resok {
                link: linktext4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for RELEASE_LOCKOWNER4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(RELEASE_LOCKOWNER4args {
                lock_owner: lock_owner4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for RELEASE_LOCKOWNER4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(RELEASE_LOCKOWNER4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for REMOVE4args {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(REMOVE4args {
                target: component4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for REMOVE4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(REMOVE4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for REMOVE4resok {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(REMOVE4resok {
                cinfo: change_info4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for RENAME4args {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(RENAME4args {
                oldname: component4::try_from(&mut *v)?,
                newname: component4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for RENAME4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(RENAME4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for RENAME4resok {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(RENAME4resok {
                source_cinfo: change_info4::try_from(&mut *v)?,
                target_cinfo: change_info4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for RENEW4args {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(RENEW4args {
                clientid: clientid4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for RENEW4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(RENEW4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for RESTOREFH4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(RESTOREFH4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for SAVEFH4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(SAVEFH4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for SECINFO4args {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(SECINFO4args {
                name: component4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for SECINFO4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(SECINFO4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for SECINFO4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_array::<secinfo4<Bytes>>(None)?))
        }
    }
    impl TryFrom<&mut Bytes> for SETATTR4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(SETATTR4args {
                stateid: stateid4::try_from(&mut *v)?,
                obj_attributes: fattr4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for SETATTR4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(SETATTR4res {
                status: nfsstat4::try_from(&mut *v)?,
                attrsset: bitmap4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for SETCLIENTID4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(SETCLIENTID4args {
                client: nfs_client_id4::try_from(&mut *v)?,
                callback: cb_client4::try_from(&mut *v)?,
                callback_ident: v.read_u32()?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for SETCLIENTID4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(SETCLIENTID4resok::try_from(&mut *v)?)
                }
                c if c == nfsstat4::NFS4ERR_CLID_INUSE as nfsstat4 => {
                    Self::NFS4ERR_CLID_INUSE(clientaddr4::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for SETCLIENTID4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(SETCLIENTID4resok {
                clientid: clientid4::try_from(&mut *v)?,
                setclientid_confirm: verifier4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for SETCLIENTID_CONFIRM4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(SETCLIENTID_CONFIRM4args {
                clientid: clientid4::try_from(&mut *v)?,
                setclientid_confirm: verifier4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for SETCLIENTID_CONFIRM4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(SETCLIENTID_CONFIRM4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for VERIFY4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(VERIFY4args {
                obj_attributes: fattr4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for VERIFY4res {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(VERIFY4res {
                status: nfsstat4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for WRITE4args<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(WRITE4args {
                stateid: stateid4::try_from(&mut *v)?,
                offset: offset4::try_from(&mut *v)?,
                stable: stable_how4::try_from(&mut *v)?,
                data: v.read_variable_bytes(None)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for WRITE4res<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let status = nfsstat4::try_from(&mut *v)?;
            Ok(match status {
                c if c == nfsstat4::NFS4_OK as nfsstat4 => {
                    Self::NFS4_OK(WRITE4resok::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for WRITE4resok<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(WRITE4resok {
                count: count4::try_from(&mut *v)?,
                committed: stable_how4::try_from(&mut *v)?,
                writeverf: verifier4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for aceflag4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for acemask4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for acetype4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for ascii_REQUIRED4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(utf8string::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for attrlist4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_bytes(None)?))
        }
    }
    impl TryFrom<&mut Bytes> for bitmap4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_array::<bitmap_inner>(None)?))
        }
    }
    impl TryFrom<&mut Bytes> for bitmap_inner {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for cb_client4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(cb_client4 {
                cb_program: v.read_u32()?,
                cb_location: clientaddr4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for change_info4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(change_info4 {
                atomic: v.read_bool()?,
                before: changeid4::try_from(&mut *v)?,
                after: changeid4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for changeid4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for clientaddr4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(clientaddr4 {
                r_netid: v.read_string(None)?,
                r_addr: v.read_string(None)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for clientid4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for component4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(utf8str_cs::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for count4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for createhow4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let mode = createmode4::try_from(&mut *v)?;
            Ok(match mode {
                c if c == createmode4::UNCHECKED4 as createmode4 => {
                    Self::UNCHECKED4(fattr4::try_from(&mut *v)?)
                }
                c if c == createmode4::GUARDED4 as createmode4 => {
                    Self::GUARDED4(fattr4::try_from(&mut *v)?)
                }
                c if c == createmode4::EXCLUSIVE4 as createmode4 => {
                    Self::EXCLUSIVE4(verifier4::try_from(&mut *v)?)
                }
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for createmode4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                0 => Self::UNCHECKED4,
                1 => Self::GUARDED4,
                2 => Self::EXCLUSIVE4,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for createtype4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let type_v = nfs_ftype4::try_from(&mut *v)?;
            Ok(match type_v {
                c if c == nfs_ftype4::NF4LNK as nfs_ftype4 => {
                    Self::NF4LNK(linktext4::try_from(&mut *v)?)
                }
                c if c == nfs_ftype4::NF4BLK as nfs_ftype4 => {
                    Self::NF4BLK(specdata4::try_from(&mut *v)?)
                }
                c if c == nfs_ftype4::NF4CHR as nfs_ftype4 => {
                    Self::NF4CHR(specdata4::try_from(&mut *v)?)
                }
                c if c == nfs_ftype4::NF4SOCK as nfs_ftype4 => Self::NF4SOCK,
                c if c == nfs_ftype4::NF4FIFO as nfs_ftype4 => Self::NF4FIFO,
                c if c == nfs_ftype4::NF4DIR as nfs_ftype4 => Self::NF4DIR,
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for dirlist4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(dirlist4 {
                entries: {
                    match v.read_u32()? {
                        0 => None,
                        1 => Some(Box::new(entry4::try_from(&mut *v)?)),
                        d => return Err(Error::UnknownOptionVariant(d)),
                    }
                },
                eof: v.read_bool()?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for entry4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(entry4 {
                cookie: nfs_cookie4::try_from(&mut *v)?,
                name: component4::try_from(&mut *v)?,
                attrs: fattr4::try_from(&mut *v)?,
                nextentry: {
                    match v.read_u32()? {
                        0 => None,
                        1 => Some(Box::new(entry4::try_from(&mut *v)?)),
                        d => return Err(Error::UnknownOptionVariant(d)),
                    }
                },
            })
        }
    }
    impl TryFrom<&mut Bytes> for exist_lock_owner4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(exist_lock_owner4 {
                lock_stateid: stateid4::try_from(&mut *v)?,
                lock_seqid: seqid4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for fattr4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(fattr4 {
                attrmask: bitmap4::try_from(&mut *v)?,
                attr_vals: attrlist4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_acl<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_array::<nfsace4<Bytes>>(None)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_aclsupport {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_archive {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_cansettime {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_case_insensitive {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_case_preserving {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_change {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(changeid4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_chown_restricted {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_fh_expire_type {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_filehandle<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfs_fh4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_fileid {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_files_avail {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_files_free {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_files_total {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_fs_locations {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(fs_locations4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_fsid {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(fsid4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_hidden {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_homogeneous {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_lease_time {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfs_lease4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_link_support {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_maxfilesize {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_maxlink {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_maxname {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_maxread {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_maxwrite {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_mimetype<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(ascii_REQUIRED4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_mode {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(mode4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_mounted_on_fileid {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_named_attr {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_no_trunc {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_numlinks {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_owner<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(utf8str_mixed::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_owner_group<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(utf8str_mixed::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_quota_avail_hard {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_quota_avail_soft {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_quota_used {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_rawdev {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(specdata4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_rdattr_error {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfsstat4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_size {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_space_avail {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_space_free {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_space_total {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_space_used {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_supported_attrs {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(bitmap4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_symlink_support {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_system {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_time_access {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfstime4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_time_access_set {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(settime4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_time_backup {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfstime4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_time_create {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfstime4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_time_delta {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfstime4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_time_metadata {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfstime4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_time_modify {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfstime4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_time_modify_set {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(settime4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_type {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(nfs_ftype4::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for fattr4_unique_handles {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bool()?))
        }
    }
    impl TryFrom<&mut Bytes> for fs_location4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(fs_location4 {
                server: v.read_variable_array::<utf8str_cis>(None)?,
                rootpath: pathname4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for fs_locations4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(fs_locations4 {
                fs_root: pathname4::try_from(&mut *v)?,
                locations: v.read_variable_array::<fs_location4>(None)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for fsid4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(fsid4 {
                major: v.read_u64()?,
                minor: v.read_u64()?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for length4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for limit_by4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                1 => Self::NFS_LIMIT_SIZE,
                2 => Self::NFS_LIMIT_BLOCKS,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for linktext4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_bytes(None)?))
        }
    }
    impl TryFrom<&mut Bytes> for lock_owner4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(lock_owner4 {
                clientid: clientid4::try_from(&mut *v)?,
                owner: v.read_variable_bytes(Some(1024))?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for locker4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let new_lock_owner = v.read_bool()?;
            Ok(match new_lock_owner {
                true => Self::TRUE(open_to_lock_owner4::try_from(&mut *v)?),
                false => Self::FALSE(exist_lock_owner4::try_from(&mut *v)?),
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for mode4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for nfs_argop4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let argop = nfs_opnum4::try_from(&mut *v)?;
            Ok(match argop {
                c if c == nfs_opnum4::OP_ACCESS as nfs_opnum4 => {
                    Self::OP_ACCESS(ACCESS4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_CLOSE as nfs_opnum4 => {
                    Self::OP_CLOSE(CLOSE4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_COMMIT as nfs_opnum4 => {
                    Self::OP_COMMIT(COMMIT4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_CREATE as nfs_opnum4 => {
                    Self::OP_CREATE(CREATE4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_DELEGPURGE as nfs_opnum4 => {
                    Self::OP_DELEGPURGE(DELEGPURGE4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_DELEGRETURN as nfs_opnum4 => {
                    Self::OP_DELEGRETURN(DELEGRETURN4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_GETATTR as nfs_opnum4 => {
                    Self::OP_GETATTR(GETATTR4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_LINK as nfs_opnum4 => {
                    Self::OP_LINK(LINK4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_LOCK as nfs_opnum4 => {
                    Self::OP_LOCK(LOCK4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_LOCKT as nfs_opnum4 => {
                    Self::OP_LOCKT(LOCKT4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_LOCKU as nfs_opnum4 => {
                    Self::OP_LOCKU(LOCKU4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_LOOKUP as nfs_opnum4 => {
                    Self::OP_LOOKUP(LOOKUP4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_NVERIFY as nfs_opnum4 => {
                    Self::OP_NVERIFY(NVERIFY4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_OPEN as nfs_opnum4 => {
                    Self::OP_OPEN(OPEN4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_OPENATTR as nfs_opnum4 => {
                    Self::OP_OPENATTR(OPENATTR4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_OPEN_CONFIRM as nfs_opnum4 => {
                    Self::OP_OPEN_CONFIRM(OPEN_CONFIRM4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_OPEN_DOWNGRADE as nfs_opnum4 => {
                    Self::OP_OPEN_DOWNGRADE(OPEN_DOWNGRADE4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_PUTFH as nfs_opnum4 => {
                    Self::OP_PUTFH(PUTFH4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_READ as nfs_opnum4 => {
                    Self::OP_READ(READ4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_READDIR as nfs_opnum4 => {
                    Self::OP_READDIR(READDIR4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_REMOVE as nfs_opnum4 => {
                    Self::OP_REMOVE(REMOVE4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_RENAME as nfs_opnum4 => {
                    Self::OP_RENAME(RENAME4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_RENEW as nfs_opnum4 => {
                    Self::OP_RENEW(RENEW4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_SECINFO as nfs_opnum4 => {
                    Self::OP_SECINFO(SECINFO4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_SETATTR as nfs_opnum4 => {
                    Self::OP_SETATTR(SETATTR4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_SETCLIENTID as nfs_opnum4 => {
                    Self::OP_SETCLIENTID(SETCLIENTID4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_SETCLIENTID_CONFIRM as nfs_opnum4 => {
                    Self::OP_SETCLIENTID_CONFIRM(SETCLIENTID_CONFIRM4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_VERIFY as nfs_opnum4 => {
                    Self::OP_VERIFY(VERIFY4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_WRITE as nfs_opnum4 => {
                    Self::OP_WRITE(WRITE4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_RELEASE_LOCKOWNER as nfs_opnum4 => {
                    Self::OP_RELEASE_LOCKOWNER(RELEASE_LOCKOWNER4args::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_GETFH as nfs_opnum4 => Self::OP_GETFH,
                c if c == nfs_opnum4::OP_LOOKUPP as nfs_opnum4 => Self::OP_LOOKUPP,
                c if c == nfs_opnum4::OP_PUTPUBFH as nfs_opnum4 => Self::OP_PUTPUBFH,
                c if c == nfs_opnum4::OP_PUTROOTFH as nfs_opnum4 => Self::OP_PUTROOTFH,
                c if c == nfs_opnum4::OP_READLINK as nfs_opnum4 => Self::OP_READLINK,
                c if c == nfs_opnum4::OP_RESTOREFH as nfs_opnum4 => Self::OP_RESTOREFH,
                c if c == nfs_opnum4::OP_SAVEFH as nfs_opnum4 => Self::OP_SAVEFH,
                c if c == nfs_opnum4::OP_ILLEGAL as nfs_opnum4 => Self::OP_ILLEGAL,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for nfs_cb_argop4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let argop = v.read_u32()?;
            Ok(match argop {
                c if c == nfs_cb_opnum4::OP_CB_GETATTR as u32 => {
                    Self::OP_CB_GETATTR(CB_GETATTR4args::try_from(&mut *v)?)
                }
                c if c == nfs_cb_opnum4::OP_CB_RECALL as u32 => {
                    Self::OP_CB_RECALL(CB_RECALL4args::try_from(&mut *v)?)
                }
                c if c == nfs_cb_opnum4::OP_CB_ILLEGAL as u32 => Self::OP_CB_ILLEGAL,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for nfs_cb_opnum4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                3 => Self::OP_CB_GETATTR,
                4 => Self::OP_CB_RECALL,
                10044 => Self::OP_CB_ILLEGAL,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for nfs_cb_resop4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let resop = v.read_u32()?;
            Ok(match resop {
                c if c == nfs_cb_opnum4::OP_CB_GETATTR as u32 => {
                    Self::OP_CB_GETATTR(CB_GETATTR4res::try_from(&mut *v)?)
                }
                c if c == nfs_cb_opnum4::OP_CB_RECALL as u32 => {
                    Self::OP_CB_RECALL(CB_RECALL4res::try_from(&mut *v)?)
                }
                c if c == nfs_cb_opnum4::OP_CB_ILLEGAL as u32 => {
                    Self::OP_CB_ILLEGAL(CB_ILLEGAL4res::try_from(&mut *v)?)
                }
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for nfs_client_id4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(nfs_client_id4 {
                verifier: verifier4::try_from(&mut *v)?,
                id: v.read_variable_bytes(Some(1024))?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for nfs_cookie4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for nfs_fh4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_bytes(None)?))
        }
    }
    impl TryFrom<&mut Bytes> for nfs_ftype4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                1 => Self::NF4REG,
                2 => Self::NF4DIR,
                3 => Self::NF4BLK,
                4 => Self::NF4CHR,
                5 => Self::NF4LNK,
                6 => Self::NF4SOCK,
                7 => Self::NF4FIFO,
                8 => Self::NF4ATTRDIR,
                9 => Self::NF4NAMEDATTR,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for nfs_lease4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for nfs_lock_type4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                1 => Self::READ_LT,
                2 => Self::WRITE_LT,
                3 => Self::READW_LT,
                4 => Self::WRITEW_LT,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for nfs_lockid4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for nfs_modified_limit4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(nfs_modified_limit4 {
                num_blocks: v.read_u32()?,
                bytes_per_block: v.read_u32()?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for nfs_opnum4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                3 => Self::OP_ACCESS,
                4 => Self::OP_CLOSE,
                5 => Self::OP_COMMIT,
                6 => Self::OP_CREATE,
                7 => Self::OP_DELEGPURGE,
                8 => Self::OP_DELEGRETURN,
                9 => Self::OP_GETATTR,
                10 => Self::OP_GETFH,
                11 => Self::OP_LINK,
                12 => Self::OP_LOCK,
                13 => Self::OP_LOCKT,
                14 => Self::OP_LOCKU,
                15 => Self::OP_LOOKUP,
                16 => Self::OP_LOOKUPP,
                17 => Self::OP_NVERIFY,
                18 => Self::OP_OPEN,
                19 => Self::OP_OPENATTR,
                20 => Self::OP_OPEN_CONFIRM,
                21 => Self::OP_OPEN_DOWNGRADE,
                22 => Self::OP_PUTFH,
                23 => Self::OP_PUTPUBFH,
                24 => Self::OP_PUTROOTFH,
                25 => Self::OP_READ,
                26 => Self::OP_READDIR,
                27 => Self::OP_READLINK,
                28 => Self::OP_REMOVE,
                29 => Self::OP_RENAME,
                30 => Self::OP_RENEW,
                31 => Self::OP_RESTOREFH,
                32 => Self::OP_SAVEFH,
                33 => Self::OP_SECINFO,
                34 => Self::OP_SETATTR,
                35 => Self::OP_SETCLIENTID,
                36 => Self::OP_SETCLIENTID_CONFIRM,
                37 => Self::OP_VERIFY,
                38 => Self::OP_WRITE,
                39 => Self::OP_RELEASE_LOCKOWNER,
                10044 => Self::OP_ILLEGAL,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for nfs_resop4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let resop = nfs_opnum4::try_from(&mut *v)?;
            Ok(match resop {
                c if c == nfs_opnum4::OP_ACCESS as nfs_opnum4 => {
                    Self::OP_ACCESS(ACCESS4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_CLOSE as nfs_opnum4 => {
                    Self::OP_CLOSE(CLOSE4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_COMMIT as nfs_opnum4 => {
                    Self::OP_COMMIT(COMMIT4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_CREATE as nfs_opnum4 => {
                    Self::OP_CREATE(CREATE4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_DELEGPURGE as nfs_opnum4 => {
                    Self::OP_DELEGPURGE(DELEGPURGE4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_DELEGRETURN as nfs_opnum4 => {
                    Self::OP_DELEGRETURN(DELEGRETURN4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_GETATTR as nfs_opnum4 => {
                    Self::OP_GETATTR(GETATTR4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_GETFH as nfs_opnum4 => {
                    Self::OP_GETFH(GETFH4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_LINK as nfs_opnum4 => {
                    Self::OP_LINK(LINK4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_LOCK as nfs_opnum4 => {
                    Self::OP_LOCK(LOCK4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_LOCKT as nfs_opnum4 => {
                    Self::OP_LOCKT(LOCKT4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_LOCKU as nfs_opnum4 => {
                    Self::OP_LOCKU(LOCKU4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_LOOKUP as nfs_opnum4 => {
                    Self::OP_LOOKUP(LOOKUP4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_LOOKUPP as nfs_opnum4 => {
                    Self::OP_LOOKUPP(LOOKUPP4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_NVERIFY as nfs_opnum4 => {
                    Self::OP_NVERIFY(NVERIFY4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_OPEN as nfs_opnum4 => {
                    Self::OP_OPEN(OPEN4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_OPENATTR as nfs_opnum4 => {
                    Self::OP_OPENATTR(OPENATTR4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_OPEN_CONFIRM as nfs_opnum4 => {
                    Self::OP_OPEN_CONFIRM(OPEN_CONFIRM4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_OPEN_DOWNGRADE as nfs_opnum4 => {
                    Self::OP_OPEN_DOWNGRADE(OPEN_DOWNGRADE4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_PUTFH as nfs_opnum4 => {
                    Self::OP_PUTFH(PUTFH4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_PUTPUBFH as nfs_opnum4 => {
                    Self::OP_PUTPUBFH(PUTPUBFH4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_PUTROOTFH as nfs_opnum4 => {
                    Self::OP_PUTROOTFH(PUTROOTFH4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_READ as nfs_opnum4 => {
                    Self::OP_READ(READ4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_READDIR as nfs_opnum4 => {
                    Self::OP_READDIR(READDIR4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_READLINK as nfs_opnum4 => {
                    Self::OP_READLINK(READLINK4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_REMOVE as nfs_opnum4 => {
                    Self::OP_REMOVE(REMOVE4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_RENAME as nfs_opnum4 => {
                    Self::OP_RENAME(RENAME4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_RENEW as nfs_opnum4 => {
                    Self::OP_RENEW(RENEW4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_RESTOREFH as nfs_opnum4 => {
                    Self::OP_RESTOREFH(RESTOREFH4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_SAVEFH as nfs_opnum4 => {
                    Self::OP_SAVEFH(SAVEFH4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_SECINFO as nfs_opnum4 => {
                    Self::OP_SECINFO(SECINFO4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_SETATTR as nfs_opnum4 => {
                    Self::OP_SETATTR(SETATTR4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_SETCLIENTID as nfs_opnum4 => {
                    Self::OP_SETCLIENTID(SETCLIENTID4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_SETCLIENTID_CONFIRM as nfs_opnum4 => {
                    Self::OP_SETCLIENTID_CONFIRM(SETCLIENTID_CONFIRM4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_VERIFY as nfs_opnum4 => {
                    Self::OP_VERIFY(VERIFY4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_WRITE as nfs_opnum4 => {
                    Self::OP_WRITE(WRITE4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_RELEASE_LOCKOWNER as nfs_opnum4 => {
                    Self::OP_RELEASE_LOCKOWNER(RELEASE_LOCKOWNER4res::try_from(&mut *v)?)
                }
                c if c == nfs_opnum4::OP_ILLEGAL as nfs_opnum4 => {
                    Self::OP_ILLEGAL(ILLEGAL4res::try_from(&mut *v)?)
                }
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for nfs_space_limit4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let limitby = limit_by4::try_from(&mut *v)?;
            Ok(match limitby {
                c if c == limit_by4::NFS_LIMIT_SIZE as limit_by4 => {
                    Self::NFS_LIMIT_SIZE(v.read_u64()?)
                }
                c if c == limit_by4::NFS_LIMIT_BLOCKS as limit_by4 => {
                    Self::NFS_LIMIT_BLOCKS(nfs_modified_limit4::try_from(&mut *v)?)
                }
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for nfsace4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(nfsace4 {
                type_v: acetype4::try_from(&mut *v)?,
                flag: aceflag4::try_from(&mut *v)?,
                access_mask: acemask4::try_from(&mut *v)?,
                who: utf8str_mixed::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for nfsstat4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                0 => Self::NFS4_OK,
                1 => Self::NFS4ERR_PERM,
                2 => Self::NFS4ERR_NOENT,
                5 => Self::NFS4ERR_IO,
                6 => Self::NFS4ERR_NXIO,
                13 => Self::NFS4ERR_ACCESS,
                17 => Self::NFS4ERR_EXIST,
                18 => Self::NFS4ERR_XDEV,
                20 => Self::NFS4ERR_NOTDIR,
                21 => Self::NFS4ERR_ISDIR,
                22 => Self::NFS4ERR_INVAL,
                27 => Self::NFS4ERR_FBIG,
                28 => Self::NFS4ERR_NOSPC,
                30 => Self::NFS4ERR_ROFS,
                31 => Self::NFS4ERR_MLINK,
                63 => Self::NFS4ERR_NAMETOOLONG,
                66 => Self::NFS4ERR_NOTEMPTY,
                69 => Self::NFS4ERR_DQUOT,
                70 => Self::NFS4ERR_STALE,
                10001 => Self::NFS4ERR_BADHANDLE,
                10003 => Self::NFS4ERR_BAD_COOKIE,
                10004 => Self::NFS4ERR_NOTSUPP,
                10005 => Self::NFS4ERR_TOOSMALL,
                10006 => Self::NFS4ERR_SERVERFAULT,
                10007 => Self::NFS4ERR_BADTYPE,
                10008 => Self::NFS4ERR_DELAY,
                10009 => Self::NFS4ERR_SAME,
                10010 => Self::NFS4ERR_DENIED,
                10011 => Self::NFS4ERR_EXPIRED,
                10012 => Self::NFS4ERR_LOCKED,
                10013 => Self::NFS4ERR_GRACE,
                10014 => Self::NFS4ERR_FHEXPIRED,
                10015 => Self::NFS4ERR_SHARE_DENIED,
                10016 => Self::NFS4ERR_WRONGSEC,
                10017 => Self::NFS4ERR_CLID_INUSE,
                10018 => Self::NFS4ERR_RESOURCE,
                10019 => Self::NFS4ERR_MOVED,
                10020 => Self::NFS4ERR_NOFILEHANDLE,
                10021 => Self::NFS4ERR_MINOR_VERS_MISMATCH,
                10022 => Self::NFS4ERR_STALE_CLIENTID,
                10023 => Self::NFS4ERR_STALE_STATEID,
                10024 => Self::NFS4ERR_OLD_STATEID,
                10025 => Self::NFS4ERR_BAD_STATEID,
                10026 => Self::NFS4ERR_BAD_SEQID,
                10027 => Self::NFS4ERR_NOT_SAME,
                10028 => Self::NFS4ERR_LOCK_RANGE,
                10029 => Self::NFS4ERR_SYMLINK,
                10030 => Self::NFS4ERR_RESTOREFH,
                10031 => Self::NFS4ERR_LEASE_MOVED,
                10032 => Self::NFS4ERR_ATTRNOTSUPP,
                10033 => Self::NFS4ERR_NO_GRACE,
                10034 => Self::NFS4ERR_RECLAIM_BAD,
                10035 => Self::NFS4ERR_RECLAIM_CONFLICT,
                10036 => Self::NFS4ERR_BADXDR,
                10037 => Self::NFS4ERR_LOCKS_HELD,
                10038 => Self::NFS4ERR_OPENMODE,
                10039 => Self::NFS4ERR_BADOWNER,
                10040 => Self::NFS4ERR_BADCHAR,
                10041 => Self::NFS4ERR_BADNAME,
                10042 => Self::NFS4ERR_BAD_RANGE,
                10043 => Self::NFS4ERR_LOCK_NOTSUPP,
                10044 => Self::NFS4ERR_OP_ILLEGAL,
                10045 => Self::NFS4ERR_DEADLOCK,
                10046 => Self::NFS4ERR_FILE_OPEN,
                10047 => Self::NFS4ERR_ADMIN_REVOKED,
                10048 => Self::NFS4ERR_CB_PATH_DOWN,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for nfstime4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(nfstime4 {
                seconds: v.read_i64()?,
                nseconds: v.read_u32()?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for offset4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u64()?))
        }
    }
    impl TryFrom<&mut Bytes> for open_claim4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let claim = open_claim_type4::try_from(&mut *v)?;
            Ok(match claim {
                c if c == open_claim_type4::CLAIM_NULL as open_claim_type4 => {
                    Self::CLAIM_NULL(component4::try_from(&mut *v)?)
                }
                c if c == open_claim_type4::CLAIM_PREVIOUS as open_claim_type4 => {
                    Self::CLAIM_PREVIOUS(open_delegation_type4::try_from(&mut *v)?)
                }
                c if c == open_claim_type4::CLAIM_DELEGATE_CUR as open_claim_type4 => {
                    Self::CLAIM_DELEGATE_CUR(open_claim_delegate_cur4::try_from(&mut *v)?)
                }
                c if c == open_claim_type4::CLAIM_DELEGATE_PREV as open_claim_type4 => {
                    Self::CLAIM_DELEGATE_PREV(component4::try_from(&mut *v)?)
                }
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for open_claim_delegate_cur4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(open_claim_delegate_cur4 {
                delegate_stateid: stateid4::try_from(&mut *v)?,
                file: component4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for open_claim_type4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                0 => Self::CLAIM_NULL,
                1 => Self::CLAIM_PREVIOUS,
                2 => Self::CLAIM_DELEGATE_CUR,
                3 => Self::CLAIM_DELEGATE_PREV,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for open_delegation4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let delegation_type = open_delegation_type4::try_from(&mut *v)?;
            Ok(match delegation_type {
                c if c == open_delegation_type4::OPEN_DELEGATE_READ as open_delegation_type4 => {
                    Self::OPEN_DELEGATE_READ(open_read_delegation4::try_from(&mut *v)?)
                }
                c if c == open_delegation_type4::OPEN_DELEGATE_WRITE as open_delegation_type4 => {
                    Self::OPEN_DELEGATE_WRITE(open_write_delegation4::try_from(&mut *v)?)
                }
                c if c == open_delegation_type4::OPEN_DELEGATE_NONE as open_delegation_type4 => {
                    Self::OPEN_DELEGATE_NONE
                }
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for open_delegation_type4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                0 => Self::OPEN_DELEGATE_NONE,
                1 => Self::OPEN_DELEGATE_READ,
                2 => Self::OPEN_DELEGATE_WRITE,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for open_owner4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(open_owner4 {
                clientid: clientid4::try_from(&mut *v)?,
                owner: v.read_variable_bytes(Some(1024))?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for open_read_delegation4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(open_read_delegation4 {
                stateid: stateid4::try_from(&mut *v)?,
                recall: v.read_bool()?,
                permissions: nfsace4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for open_to_lock_owner4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(open_to_lock_owner4 {
                open_seqid: seqid4::try_from(&mut *v)?,
                open_stateid: stateid4::try_from(&mut *v)?,
                lock_seqid: seqid4::try_from(&mut *v)?,
                lock_owner: lock_owner4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for open_write_delegation4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(open_write_delegation4 {
                stateid: stateid4::try_from(&mut *v)?,
                recall: v.read_bool()?,
                space_limit: nfs_space_limit4::try_from(&mut *v)?,
                permissions: nfsace4::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for openflag4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let opentype = opentype4::try_from(&mut *v)?;
            Ok(match opentype {
                c if c == opentype4::OPEN4_NOCREATE as opentype4 => Self::OPEN4_NOCREATE,
                c if c == opentype4::OPEN4_CREATE as opentype4 => {
                    Self::OPEN4_CREATE(createhow4::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for opentype4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let d = v.read_i32()?;
            debug!("opentype4 {:?}", d);
            Ok(match d {
                0 => Self::OPEN4_NOCREATE,
                1 => Self::OPEN4_CREATE,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for pathname4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_array::<component4>(None)?))
        }
    }
    impl TryFrom<&mut Bytes> for qop4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for rpc_gss_svc_t {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                1 => Self::RPC_GSS_SVC_NONE,
                2 => Self::RPC_GSS_SVC_INTEGRITY,
                3 => Self::RPC_GSS_SVC_PRIVACY,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for rpcsec_gss_info<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(rpcsec_gss_info {
                oid: sec_oid4::try_from(&mut *v)?,
                qop: qop4::try_from(&mut *v)?,
                service: rpc_gss_svc_t::try_from(&mut *v)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for sec_oid4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_bytes(None)?))
        }
    }
    impl TryFrom<&mut Bytes> for secinfo4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let flavor = v.read_u32()?;
            Ok(match flavor {
                RPCSEC_GSS => Self::RPCSEC_GSS(rpcsec_gss_info::try_from(&mut *v)?),
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for seqid4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_u32()?))
        }
    }
    impl TryFrom<&mut Bytes> for settime4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            let set_it = time_how4::try_from(&mut *v)?;
            Ok(match set_it {
                c if c == time_how4::SET_TO_CLIENT_TIME4 as time_how4 => {
                    Self::SET_TO_CLIENT_TIME4(nfstime4::try_from(&mut *v)?)
                }
                _ => Self::default,
            })
        }
    }
    impl TryFrom<&mut Bytes> for specdata4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(specdata4 {
                specdata1: v.read_u32()?,
                specdata2: v.read_u32()?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for stable_how4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                0 => Self::UNSTABLE4,
                1 => Self::DATA_SYNC4,
                2 => Self::FILE_SYNC4,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for stateid4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(stateid4 {
                seqid: v.read_u32()?,
                other: v.read_bytes(12)?,
            })
        }
    }
    impl TryFrom<&mut Bytes> for time_how4 {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(match v.read_i32()? {
                0 => Self::SET_TO_SERVER_TIME4,
                1 => Self::SET_TO_CLIENT_TIME4,
                d => return Err(Error::UnknownVariant(d as i32)),
            })
        }
    }
    impl TryFrom<&mut Bytes> for utf8str_cis {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_string(None)?))
        }
    }
    impl TryFrom<&mut Bytes> for utf8str_cs {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_string(None)?))
        }
    }
    impl TryFrom<&mut Bytes> for utf8str_mixed<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(utf8string::try_from(&mut *v)?))
        }
    }
    impl TryFrom<&mut Bytes> for utf8string<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_variable_bytes(None)?))
        }
    }
    impl TryFrom<&mut Bytes> for verifier4<Bytes> {
        type Error = Error;

        fn try_from(mut v: &mut Bytes) -> Result<Self, Self::Error> {
            Ok(Self(v.read_bytes(8)?))
        }
    }
    impl WireSize for ACCESS4args {
        fn wire_size(&self) -> usize {
            self.access.wire_size() + 0
        }
    }
    impl WireSize for ACCESS4res {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for ACCESS4resok {
        fn wire_size(&self) -> usize {
            self.supported.wire_size() + self.access.wire_size() + 0
        }
    }
    impl WireSize for CB_COMPOUND4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.tag.wire_size()
                + self.minorversion.wire_size()
                + self.callback_ident.wire_size()
                + self.argarray.wire_size()
                + 0
        }
    }
    impl WireSize for CB_COMPOUND4res<Bytes> {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + self.tag.wire_size() + self.resarray.wire_size() + 0
        }
    }
    impl WireSize for CB_GETATTR4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.fh.wire_size() + self.attr_request.wire_size() + 0
        }
    }
    impl WireSize for CB_GETATTR4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for CB_GETATTR4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.obj_attributes.wire_size() + 0
        }
    }
    impl WireSize for CB_ILLEGAL4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for CB_RECALL4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.stateid.wire_size() + self.truncate.wire_size() + self.fh.wire_size() + 0
        }
    }
    impl WireSize for CB_RECALL4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for CLOSE4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.seqid.wire_size() + self.open_stateid.wire_size() + 0
        }
    }
    impl WireSize for CLOSE4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for COMMIT4args {
        fn wire_size(&self) -> usize {
            self.offset.wire_size() + self.count.wire_size() + 0
        }
    }
    impl WireSize for COMMIT4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for COMMIT4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.writeverf.wire_size() + 0
        }
    }
    impl WireSize for COMPOUND4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.tag.wire_size() + self.minorversion.wire_size() + self.argarray.wire_size() + 0
        }
    }
    impl WireSize for COMPOUND4res<Bytes> {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + self.tag.wire_size() + self.resarray.wire_size() + 0
        }
    }
    impl WireSize for CREATE4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.objtype.wire_size() + self.objname.wire_size() + self.createattrs.wire_size() + 0
        }
    }
    impl WireSize for CREATE4res {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for CREATE4resok {
        fn wire_size(&self) -> usize {
            self.cinfo.wire_size() + self.attrset.wire_size() + 0
        }
    }
    impl WireSize for DELEGPURGE4args {
        fn wire_size(&self) -> usize {
            self.clientid.wire_size() + 0
        }
    }
    impl WireSize for DELEGPURGE4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for DELEGRETURN4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.deleg_stateid.wire_size() + 0
        }
    }
    impl WireSize for DELEGRETURN4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for GETATTR4args {
        fn wire_size(&self) -> usize {
            self.attr_request.wire_size() + 0
        }
    }
    impl WireSize for GETATTR4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for GETATTR4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.obj_attributes.wire_size() + 0
        }
    }
    impl WireSize for GETFH4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for GETFH4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.object.wire_size() + 0
        }
    }
    impl WireSize for ILLEGAL4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for LINK4args {
        fn wire_size(&self) -> usize {
            self.newname.wire_size() + 0
        }
    }
    impl WireSize for LINK4res {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for LINK4resok {
        fn wire_size(&self) -> usize {
            self.cinfo.wire_size() + 0
        }
    }
    impl WireSize for LOCK4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.locktype.wire_size()
                + self.reclaim.wire_size()
                + self.offset.wire_size()
                + self.length.wire_size()
                + self.locker.wire_size()
                + 0
        }
    }
    impl WireSize for LOCK4denied<Bytes> {
        fn wire_size(&self) -> usize {
            self.offset.wire_size()
                + self.length.wire_size()
                + self.locktype.wire_size()
                + self.owner.wire_size()
                + 0
        }
    }
    impl WireSize for LOCK4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::NFS4ERR_DENIED(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for LOCK4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.lock_stateid.wire_size() + 0
        }
    }
    impl WireSize for LOCKT4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.locktype.wire_size()
                + self.offset.wire_size()
                + self.length.wire_size()
                + self.owner.wire_size()
                + 0
        }
    }
    impl WireSize for LOCKT4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4ERR_DENIED(inner) => inner.wire_size(),
                Self::NFS4_OK => 0,
                Self::default => 0,
            }
        }
    }
    impl WireSize for LOCKU4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.locktype.wire_size()
                + self.seqid.wire_size()
                + self.lock_stateid.wire_size()
                + self.offset.wire_size()
                + self.length.wire_size()
                + 0
        }
    }
    impl WireSize for LOCKU4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for LOOKUP4args {
        fn wire_size(&self) -> usize {
            self.objname.wire_size() + 0
        }
    }
    impl WireSize for LOOKUP4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for LOOKUPP4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for NVERIFY4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.obj_attributes.wire_size() + 0
        }
    }
    impl WireSize for NVERIFY4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for OPEN4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.seqid.wire_size()
                + self.share_access.wire_size()
                + self.share_deny.wire_size()
                + self.owner.wire_size()
                + self.openhow.wire_size()
                + self.claim.wire_size()
                + 4
        }
    }
    impl WireSize for OPEN4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for OPEN4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.stateid.wire_size()
                + self.cinfo.wire_size()
                + self.rflags.wire_size()
                + self.attrset.wire_size()
                + self.delegation.wire_size()
                + 0
        }
    }
    impl WireSize for OPENATTR4args {
        fn wire_size(&self) -> usize {
            self.createdir.wire_size() + 0
        }
    }
    impl WireSize for OPENATTR4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for OPEN_CONFIRM4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.open_stateid.wire_size() + self.seqid.wire_size() + 0
        }
    }
    impl WireSize for OPEN_CONFIRM4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for OPEN_CONFIRM4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.open_stateid.wire_size() + 0
        }
    }
    impl WireSize for OPEN_DOWNGRADE4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.open_stateid.wire_size()
                + self.seqid.wire_size()
                + self.share_access.wire_size()
                + self.share_deny.wire_size()
                + 0
        }
    }
    impl WireSize for OPEN_DOWNGRADE4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for OPEN_DOWNGRADE4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.open_stateid.wire_size() + 0
        }
    }
    impl WireSize for PUTFH4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.object.wire_size() + 0
        }
    }
    impl WireSize for PUTFH4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for PUTPUBFH4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for PUTROOTFH4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for READ4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.stateid.wire_size() + self.offset.wire_size() + self.count.wire_size() + 0
        }
    }
    impl WireSize for READ4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for READ4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.eof.wire_size() + self.data.wire_size() + pad_length(self.data.wire_size()) + 0
        }
    }
    impl WireSize for READDIR4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.cookie.wire_size()
                + self.cookieverf.wire_size()
                + self.dircount.wire_size()
                + self.maxcount.wire_size()
                + self.attr_request.wire_size()
                + 0
        }
    }
    impl WireSize for READDIR4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for READDIR4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.cookieverf.wire_size() + self.reply.wire_size() + 0
        }
    }
    impl WireSize for READLINK4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for READLINK4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.link.wire_size() + 0
        }
    }
    impl WireSize for RELEASE_LOCKOWNER4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.lock_owner.wire_size() + 0
        }
    }
    impl WireSize for RELEASE_LOCKOWNER4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for REMOVE4args {
        fn wire_size(&self) -> usize {
            self.target.wire_size() + 0
        }
    }
    impl WireSize for REMOVE4res {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for REMOVE4resok {
        fn wire_size(&self) -> usize {
            self.cinfo.wire_size() + 0
        }
    }
    impl WireSize for RENAME4args {
        fn wire_size(&self) -> usize {
            self.oldname.wire_size() + self.newname.wire_size() + 0
        }
    }
    impl WireSize for RENAME4res {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for RENAME4resok {
        fn wire_size(&self) -> usize {
            self.source_cinfo.wire_size() + self.target_cinfo.wire_size() + 0
        }
    }
    impl WireSize for RENEW4args {
        fn wire_size(&self) -> usize {
            self.clientid.wire_size() + 0
        }
    }
    impl WireSize for RENEW4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for RESTOREFH4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for SAVEFH4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for SECINFO4args {
        fn wire_size(&self) -> usize {
            self.name.wire_size() + 0
        }
    }
    impl WireSize for SECINFO4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for SECINFO4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for SETATTR4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.stateid.wire_size() + self.obj_attributes.wire_size() + 0
        }
    }
    impl WireSize for SETATTR4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + self.attrsset.wire_size() + 0
        }
    }
    impl WireSize for SETCLIENTID4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.client.wire_size()
                + self.callback.wire_size()
                + self.callback_ident.wire_size()
                + 0
        }
    }
    impl WireSize for SETCLIENTID4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::NFS4ERR_CLID_INUSE(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for SETCLIENTID4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.clientid.wire_size() + self.setclientid_confirm.wire_size() + 0
        }
    }
    impl WireSize for SETCLIENTID_CONFIRM4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.clientid.wire_size() + self.setclientid_confirm.wire_size() + 0
        }
    }
    impl WireSize for SETCLIENTID_CONFIRM4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for VERIFY4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.obj_attributes.wire_size() + 0
        }
    }
    impl WireSize for VERIFY4res {
        fn wire_size(&self) -> usize {
            self.status.wire_size() + 0
        }
    }
    impl WireSize for WRITE4args<Bytes> {
        fn wire_size(&self) -> usize {
            self.stateid.wire_size()
                + self.offset.wire_size()
                + self.stable.wire_size()
                + self.data.wire_size()
                + pad_length(self.data.wire_size())
                + 4
        }
    }
    impl WireSize for WRITE4res<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS4_OK(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for WRITE4resok<Bytes> {
        fn wire_size(&self) -> usize {
            self.count.wire_size() + self.committed.wire_size() + self.writeverf.wire_size() + 0
        }
    }
    impl WireSize for aceflag4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for acemask4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for acetype4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for ascii_REQUIRED4<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for attrlist4<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size() + pad_length(self.0.wire_size()) + 4
        }
    }
    impl WireSize for bitmap4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for bitmap_inner {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for cb_client4 {
        fn wire_size(&self) -> usize {
            self.cb_program.wire_size() + self.cb_location.wire_size() + 0
        }
    }
    impl WireSize for change_info4 {
        fn wire_size(&self) -> usize {
            self.atomic.wire_size() + self.before.wire_size() + self.after.wire_size() + 0
        }
    }
    impl WireSize for changeid4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for clientaddr4 {
        fn wire_size(&self) -> usize {
            self.r_netid.wire_size() + self.r_addr.wire_size() + 0
        }
    }
    impl WireSize for clientid4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for component4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for count4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for createhow4<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::UNCHECKED4(inner) => inner.wire_size(),
                Self::GUARDED4(inner) => inner.wire_size(),
                Self::EXCLUSIVE4(inner) => inner.wire_size(),
            }
        }
    }
    impl WireSize for createmode4 {
        fn wire_size(&self) -> usize {
            4
        }
    }
    impl WireSize for createtype4<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NF4LNK(inner) => inner.wire_size(),
                Self::NF4BLK(inner) => inner.wire_size(),
                Self::NF4CHR(inner) => inner.wire_size(),
                Self::NF4SOCK => 0,
                Self::NF4FIFO => 0,
                Self::NF4DIR => 0,
                Self::default => 0,
            }
        }
    }
    impl WireSize for dirlist4<Bytes> {
        fn wire_size(&self) -> usize {
            self.entries.wire_size() + self.eof.wire_size() + 0
        }
    }
    impl WireSize for entry4<Bytes> {
        fn wire_size(&self) -> usize {
            self.cookie.wire_size()
                + self.name.wire_size()
                + self.attrs.wire_size()
                + self.nextentry.wire_size()
                + 0
        }
    }
    impl WireSize for exist_lock_owner4<Bytes> {
        fn wire_size(&self) -> usize {
            self.lock_stateid.wire_size() + self.lock_seqid.wire_size() + 0
        }
    }
    impl WireSize for fattr4<Bytes> {
        fn wire_size(&self) -> usize {
            self.attrmask.wire_size() + self.attr_vals.wire_size() + 0
        }
    }
    impl WireSize for fattr4_acl<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_aclsupport {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_archive {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_cansettime {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_case_insensitive {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_case_preserving {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_change {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_chown_restricted {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_fh_expire_type {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_filehandle<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_fileid {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_files_avail {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_files_free {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_files_total {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_fs_locations {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_fsid {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_hidden {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_homogeneous {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_lease_time {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_link_support {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_maxfilesize {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_maxlink {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_maxname {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_maxread {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_maxwrite {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_mimetype<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_mode {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_mounted_on_fileid {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_named_attr {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_no_trunc {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_numlinks {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_owner<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_owner_group<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_quota_avail_hard {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_quota_avail_soft {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_quota_used {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_rawdev {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_rdattr_error {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_size {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_space_avail {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_space_free {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_space_total {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_space_used {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_supported_attrs {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_symlink_support {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_system {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_time_access {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_time_access_set {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_time_backup {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_time_create {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_time_delta {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_time_metadata {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_time_modify {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_time_modify_set {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_type {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fattr4_unique_handles {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for fs_location4 {
        fn wire_size(&self) -> usize {
            self.server.wire_size() + self.rootpath.wire_size() + 0
        }
    }
    impl WireSize for fs_locations4 {
        fn wire_size(&self) -> usize {
            self.fs_root.wire_size() + self.locations.wire_size() + 0
        }
    }
    impl WireSize for fsid4 {
        fn wire_size(&self) -> usize {
            self.major.wire_size() + self.minor.wire_size() + 0
        }
    }
    impl WireSize for length4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for limit_by4 {
        fn wire_size(&self) -> usize {
            4
        }
    }
    impl WireSize for linktext4<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size() + pad_length(self.0.wire_size()) + 4
        }
    }
    impl WireSize for lock_owner4<Bytes> {
        fn wire_size(&self) -> usize {
            self.clientid.wire_size()
                + self.owner.wire_size()
                + pad_length(self.owner.wire_size())
                + 0
        }
    }
    impl WireSize for locker4<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::TRUE(inner) => inner.wire_size(),
                Self::FALSE(inner) => inner.wire_size(),
            }
        }
    }
    impl WireSize for mode4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for nfs_argop4<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::OP_ACCESS(inner) => inner.wire_size(),
                Self::OP_CLOSE(inner) => inner.wire_size(),
                Self::OP_COMMIT(inner) => inner.wire_size(),
                Self::OP_CREATE(inner) => inner.wire_size(),
                Self::OP_DELEGPURGE(inner) => inner.wire_size(),
                Self::OP_DELEGRETURN(inner) => inner.wire_size(),
                Self::OP_GETATTR(inner) => inner.wire_size(),
                Self::OP_LINK(inner) => inner.wire_size(),
                Self::OP_LOCK(inner) => inner.wire_size(),
                Self::OP_LOCKT(inner) => inner.wire_size(),
                Self::OP_LOCKU(inner) => inner.wire_size(),
                Self::OP_LOOKUP(inner) => inner.wire_size(),
                Self::OP_NVERIFY(inner) => inner.wire_size(),
                Self::OP_OPEN(inner) => inner.wire_size(),
                Self::OP_OPENATTR(inner) => inner.wire_size(),
                Self::OP_OPEN_CONFIRM(inner) => inner.wire_size(),
                Self::OP_OPEN_DOWNGRADE(inner) => inner.wire_size(),
                Self::OP_PUTFH(inner) => inner.wire_size(),
                Self::OP_READ(inner) => inner.wire_size(),
                Self::OP_READDIR(inner) => inner.wire_size(),
                Self::OP_REMOVE(inner) => inner.wire_size(),
                Self::OP_RENAME(inner) => inner.wire_size(),
                Self::OP_RENEW(inner) => inner.wire_size(),
                Self::OP_SECINFO(inner) => inner.wire_size(),
                Self::OP_SETATTR(inner) => inner.wire_size(),
                Self::OP_SETCLIENTID(inner) => inner.wire_size(),
                Self::OP_SETCLIENTID_CONFIRM(inner) => inner.wire_size(),
                Self::OP_VERIFY(inner) => inner.wire_size(),
                Self::OP_WRITE(inner) => inner.wire_size(),
                Self::OP_RELEASE_LOCKOWNER(inner) => inner.wire_size(),
                Self::OP_GETFH => 0,
                Self::OP_LOOKUPP => 0,
                Self::OP_PUTPUBFH => 0,
                Self::OP_PUTROOTFH => 0,
                Self::OP_READLINK => 0,
                Self::OP_RESTOREFH => 0,
                Self::OP_SAVEFH => 0,
                Self::OP_ILLEGAL => 0,
            }
        }
    }
    impl WireSize for nfs_cb_argop4<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::OP_CB_GETATTR(inner) => inner.wire_size(),
                Self::OP_CB_RECALL(inner) => inner.wire_size(),
                Self::OP_CB_ILLEGAL => 0,
            }
        }
    }
    impl WireSize for nfs_cb_opnum4 {
        fn wire_size(&self) -> usize {
            4
        }
    }
    impl WireSize for nfs_cb_resop4<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::OP_CB_GETATTR(inner) => inner.wire_size(),
                Self::OP_CB_RECALL(inner) => inner.wire_size(),
                Self::OP_CB_ILLEGAL(inner) => inner.wire_size(),
            }
        }
    }
    impl WireSize for nfs_client_id4<Bytes> {
        fn wire_size(&self) -> usize {
            self.verifier.wire_size() + self.id.wire_size() + pad_length(self.id.wire_size()) + 0
        }
    }
    impl WireSize for nfs_cookie4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for nfs_fh4<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size() + pad_length(self.0.wire_size()) + 4
        }
    }
    impl WireSize for nfs_ftype4 {
        fn wire_size(&self) -> usize {
            4
        }
    }
    impl WireSize for nfs_lease4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for nfs_lock_type4 {
        fn wire_size(&self) -> usize {
            4
        }
    }
    impl WireSize for nfs_lockid4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for nfs_modified_limit4 {
        fn wire_size(&self) -> usize {
            self.num_blocks.wire_size() + self.bytes_per_block.wire_size() + 0
        }
    }
    impl WireSize for nfs_opnum4 {
        fn wire_size(&self) -> usize {
            4
        }
    }
    impl WireSize for nfs_resop4<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::OP_ACCESS(inner) => inner.wire_size(),
                Self::OP_CLOSE(inner) => inner.wire_size(),
                Self::OP_COMMIT(inner) => inner.wire_size(),
                Self::OP_CREATE(inner) => inner.wire_size(),
                Self::OP_DELEGPURGE(inner) => inner.wire_size(),
                Self::OP_DELEGRETURN(inner) => inner.wire_size(),
                Self::OP_GETATTR(inner) => inner.wire_size(),
                Self::OP_GETFH(inner) => inner.wire_size(),
                Self::OP_LINK(inner) => inner.wire_size(),
                Self::OP_LOCK(inner) => inner.wire_size(),
                Self::OP_LOCKT(inner) => inner.wire_size(),
                Self::OP_LOCKU(inner) => inner.wire_size(),
                Self::OP_LOOKUP(inner) => inner.wire_size(),
                Self::OP_LOOKUPP(inner) => inner.wire_size(),
                Self::OP_NVERIFY(inner) => inner.wire_size(),
                Self::OP_OPEN(inner) => inner.wire_size(),
                Self::OP_OPENATTR(inner) => inner.wire_size(),
                Self::OP_OPEN_CONFIRM(inner) => inner.wire_size(),
                Self::OP_OPEN_DOWNGRADE(inner) => inner.wire_size(),
                Self::OP_PUTFH(inner) => inner.wire_size(),
                Self::OP_PUTPUBFH(inner) => inner.wire_size(),
                Self::OP_PUTROOTFH(inner) => inner.wire_size(),
                Self::OP_READ(inner) => inner.wire_size(),
                Self::OP_READDIR(inner) => inner.wire_size(),
                Self::OP_READLINK(inner) => inner.wire_size(),
                Self::OP_REMOVE(inner) => inner.wire_size(),
                Self::OP_RENAME(inner) => inner.wire_size(),
                Self::OP_RENEW(inner) => inner.wire_size(),
                Self::OP_RESTOREFH(inner) => inner.wire_size(),
                Self::OP_SAVEFH(inner) => inner.wire_size(),
                Self::OP_SECINFO(inner) => inner.wire_size(),
                Self::OP_SETATTR(inner) => inner.wire_size(),
                Self::OP_SETCLIENTID(inner) => inner.wire_size(),
                Self::OP_SETCLIENTID_CONFIRM(inner) => inner.wire_size(),
                Self::OP_VERIFY(inner) => inner.wire_size(),
                Self::OP_WRITE(inner) => inner.wire_size(),
                Self::OP_RELEASE_LOCKOWNER(inner) => inner.wire_size(),
                Self::OP_ILLEGAL(inner) => inner.wire_size(),
            }
        }
    }
    impl WireSize for nfs_space_limit4 {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::NFS_LIMIT_SIZE(inner) => inner.wire_size(),
                Self::NFS_LIMIT_BLOCKS(inner) => inner.wire_size(),
            }
        }
    }
    impl WireSize for nfsace4<Bytes> {
        fn wire_size(&self) -> usize {
            self.type_v.wire_size()
                + self.flag.wire_size()
                + self.access_mask.wire_size()
                + self.who.wire_size()
                + 0
        }
    }
    impl WireSize for nfsstat4 {
        fn wire_size(&self) -> usize {
            4
        }
    }
    impl WireSize for nfstime4 {
        fn wire_size(&self) -> usize {
            self.seconds.wire_size() + self.nseconds.wire_size() + 0
        }
    }
    impl WireSize for offset4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for open_claim4<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::CLAIM_NULL(inner) => inner.wire_size(),
                Self::CLAIM_PREVIOUS(inner) => inner.wire_size(),
                Self::CLAIM_DELEGATE_CUR(inner) => inner.wire_size(),
                Self::CLAIM_DELEGATE_PREV(inner) => inner.wire_size(),
            }
        }
    }
    impl WireSize for open_claim_delegate_cur4<Bytes> {
        fn wire_size(&self) -> usize {
            self.delegate_stateid.wire_size() + self.file.wire_size() + 0
        }
    }
    impl WireSize for open_claim_type4 {
        fn wire_size(&self) -> usize {
            4
        }
    }
    impl WireSize for open_delegation4<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::OPEN_DELEGATE_READ(inner) => inner.wire_size(),
                Self::OPEN_DELEGATE_WRITE(inner) => inner.wire_size(),
                Self::OPEN_DELEGATE_NONE => 0,
            }
        }
    }
    impl WireSize for open_delegation_type4 {
        fn wire_size(&self) -> usize {
            4
        }
    }
    impl WireSize for open_owner4<Bytes> {
        fn wire_size(&self) -> usize {
            self.clientid.wire_size()
                + self.owner.wire_size()
                + pad_length(self.owner.wire_size())
                + 0
        }
    }
    impl WireSize for open_read_delegation4<Bytes> {
        fn wire_size(&self) -> usize {
            self.stateid.wire_size() + self.recall.wire_size() + self.permissions.wire_size() + 0
        }
    }
    impl WireSize for open_to_lock_owner4<Bytes> {
        fn wire_size(&self) -> usize {
            self.open_seqid.wire_size()
                + self.open_stateid.wire_size()
                + self.lock_seqid.wire_size()
                + self.lock_owner.wire_size()
                + 0
        }
    }
    impl WireSize for open_write_delegation4<Bytes> {
        fn wire_size(&self) -> usize {
            self.stateid.wire_size()
                + self.recall.wire_size()
                + self.space_limit.wire_size()
                + self.permissions.wire_size()
                + 0
        }
    }
    impl WireSize for openflag4<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::OPEN4_NOCREATE => 0,
                Self::OPEN4_CREATE(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for opentype4 {
        fn wire_size(&self) -> usize {
            4
        }
    }
    impl WireSize for pathname4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for qop4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for rpc_gss_svc_t {
        fn wire_size(&self) -> usize {
            4
        }
    }
    impl WireSize for rpcsec_gss_info<Bytes> {
        fn wire_size(&self) -> usize {
            self.oid.wire_size() + self.qop.wire_size() + self.service.wire_size() + 0
        }
    }
    impl WireSize for sec_oid4<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size() + pad_length(self.0.wire_size()) + 4
        }
    }
    impl WireSize for secinfo4<Bytes> {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::RPCSEC_GSS(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for seqid4 {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for settime4 {
        fn wire_size(&self) -> usize {
            4 + match self {
                Self::SET_TO_CLIENT_TIME4(inner) => inner.wire_size(),
                Self::default => 0,
            }
        }
    }
    impl WireSize for specdata4 {
        fn wire_size(&self) -> usize {
            self.specdata1.wire_size() + self.specdata2.wire_size() + 0
        }
    }
    impl WireSize for stable_how4 {
        fn wire_size(&self) -> usize {
            4
        }
    }
    impl WireSize for stateid4<Bytes> {
        fn wire_size(&self) -> usize {
            self.seqid.wire_size() + self.other.wire_size() + pad_length(self.other.wire_size()) + 0
        }
    }
    impl WireSize for time_how4 {
        fn wire_size(&self) -> usize {
            4
        }
    }
    impl WireSize for utf8str_cis {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for utf8str_cs {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for utf8str_mixed<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size()
        }
    }
    impl WireSize for utf8string<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size() + pad_length(self.0.wire_size()) + 4
        }
    }
    impl WireSize for verifier4<Bytes> {
        fn wire_size(&self) -> usize {
            self.0.wire_size() + pad_length(self.0.wire_size())
        }
    }
}
