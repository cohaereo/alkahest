use std::{
    fmt::{Debug, Display, Formatter},
    ops::Deref,
};

use alkahest_pm::package_manager;
use binrw::{BinRead, BinReaderExt};
use destiny_pkg::{TagHash, TagHash64};
use tiger_parse::{dpkg::PackageManagerExt, TigerReadable};

#[derive(Clone)]
pub struct Tag<T: TigerReadable>(pub T, TagHash);

impl<T: TigerReadable> TigerReadable for Tag<T> {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let tag = TagHash::read_ds_endian(reader, endian)?;
        Ok(Tag(package_manager().read_tag_struct(tag)?, tag))
    }

    const ZEROCOPY: bool = false;
    const SIZE: usize = TagHash::SIZE;
}

impl<T: TigerReadable> Tag<T> {
    pub fn taghash(&self) -> TagHash {
        self.1
    }
}

impl<T: TigerReadable> Deref for Tag<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TigerReadable + Debug> Debug for Tag<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// TODO(cohae): Custom reader once new tag parser comes around
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum WideHash {
    Hash32(TagHash),
    Hash64(TagHash64),
}

impl WideHash {
    /// Key that is safe to use for caching/lookup tables
    pub fn key(&self) -> u64 {
        match self {
            WideHash::Hash32(v) => v.0 as u64,
            WideHash::Hash64(v) => v.0,
        }
    }

    /// Will lookup hash64 in package managers's h64 table in the case of a 64 bit hash
    /// Falls back to TagHash::NONE if not found
    pub fn hash32(&self) -> TagHash {
        self.hash32_checked().unwrap_or(TagHash::NONE)
    }

    /// Will lookup hash64 in package managers's h64 table in the case of a 64 bit hash
    /// Returns None if the hash is not found or null in case of a 32 bit hash
    pub fn hash32_checked(&self) -> Option<TagHash> {
        match self {
            WideHash::Hash32(v) => {
                if v.is_some() {
                    Some(*v)
                } else {
                    None
                }
            }
            WideHash::Hash64(v) => package_manager().hash64_table.get(&v.0).map(|v| v.hash32),
        }
    }

    pub fn is_some(&self) -> bool {
        match self {
            WideHash::Hash32(h) => h.is_some(),
            // TODO(cohae): Double check this
            WideHash::Hash64(h) => h.0 != 0 && h.0 != u64::MAX,
        }
    }
}

impl Debug for WideHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WideHash::Hash32(h) => f.write_fmt(format_args!("Hash32({:08X})", h.0.to_be())),
            WideHash::Hash64(h) => f.write_fmt(format_args!("Hash64({:016X})", h.0.to_be())),
        }
    }
}

impl Display for WideHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WideHash::Hash32(h) => f.write_fmt(format_args!("{:08X}", h.0.to_be())),
            WideHash::Hash64(h) => f.write_fmt(format_args!("{:016X}", h.0.to_be())),
        }
    }
}

impl std::hash::Hash for WideHash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.key());
    }
}

impl From<WideHash> for TagHash {
    fn from(val: WideHash) -> Self {
        val.hash32()
    }
}

impl From<TagHash> for WideHash {
    fn from(val: TagHash) -> Self {
        WideHash::Hash32(val)
    }
}

impl nohash_hasher::IsEnabled for WideHash {}

impl BinRead for WideHash {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let hash32: TagHash = reader.read_type(endian)?;
        let is_hash32: u32 = reader.read_type(endian)?;
        let hash64: TagHash64 = reader.read_type(endian)?;

        if is_hash32 != 0 {
            Ok(WideHash::Hash32(hash32))
        } else {
            Ok(WideHash::Hash64(hash64))
        }
    }
}

impl TigerReadable for WideHash {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let hash32: TagHash = TigerReadable::read_ds_endian(reader, endian)?;
        let is_hash32: u32 = TigerReadable::read_ds_endian(reader, endian)?;
        let hash64: TagHash64 = TigerReadable::read_ds_endian(reader, endian)?;

        if is_hash32 != 0 {
            Ok(WideHash::Hash32(hash32))
        } else {
            Ok(WideHash::Hash64(hash64))
        }
    }

    const ZEROCOPY: bool = false;
    const SIZE: usize = 16;
}

#[derive(Clone)]
pub struct WideTag<T: TigerReadable>(pub T, pub TagHash);

impl<T: TigerReadable> TigerReadable for WideTag<T> {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let tag = WideHash::read_ds_endian(reader, endian)?;
        match tag {
            WideHash::Hash32(h) => Ok(WideTag(package_manager().read_tag_struct(h)?, h)),
            WideHash::Hash64(h) => Ok(WideTag(
                package_manager().read_tag64_struct(h)?,
                tag.hash32(),
            )),
        }
    }

    const ZEROCOPY: bool = false;
    const SIZE: usize = TagHash::SIZE;
}

impl<T: TigerReadable> Deref for WideTag<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TigerReadable + Debug> Debug for WideTag<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
