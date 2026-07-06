use std::fmt::{Debug, Display, Formatter};

use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{package_manager, TagHash, TagHash64};

#[derive(Clone)]
pub struct Tag<T: TigerReadable>(pub T, TagHash);

impl<T: TigerReadable> TigerReadable for Tag<T> {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let tag = TagHash::read_ds_endian(reader, endian)?;
        if tag.is_none() {
            return Err(tiger_parse::Error::TagReadFailed(
                "Attempted to read Tag with an unset TagHash (0xFFFFFFFF). Perhaps you meant to \
                 use an OptionalTag<T>?"
                    .to_string(),
            ));
        }
        Ok(Tag(package_manager().read_tag_struct(tag)?, tag))
    }

    const SIZE: usize = TagHash::SIZE;
}

impl<T: TigerReadable> Tag<T> {
    pub fn taghash(&self) -> TagHash {
        self.1
    }
}

impl<T: TigerReadable> std::ops::Deref for Tag<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TigerReadable + Debug> Debug for Tag<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Tag({}, ", self.1))?;
        self.0.fmt(f)?;
        f.write_str(")")
    }
}

#[derive(Clone)]
pub struct OptionalTag<T: TigerReadable>(pub Option<T>, TagHash);

impl<T: TigerReadable> TigerReadable for OptionalTag<T> {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let tag = TagHash::read_ds_endian(reader, endian)?;
        let data = if tag.is_some() {
            Some(package_manager().read_tag_struct::<T>(tag)?)
        } else {
            None
        };

        Ok(Self(data, tag))
    }

    const SIZE: usize = TagHash::SIZE;
}

impl<T: TigerReadable> std::ops::Deref for OptionalTag<T> {
    type Target = Option<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TigerReadable> OptionalTag<T> {
    pub fn taghash(&self) -> TagHash {
        self.1
    }
}

impl<T: TigerReadable + Debug> Debug for OptionalTag<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("OptionalTag({}, ", self.1))?;
        self.0.fmt(f)?;
        f.write_str(")")
    }
}

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
            WideHash::Hash32(v) => v.is_some().then_some(*v),
            WideHash::Hash64(v) => package_manager()
                .lookup
                .tag64_entries
                .get(&v.0)
                .map(|v| v.hash32),
        }
    }

    pub fn is_some(&self) -> bool {
        match self {
            WideHash::Hash32(h) => h.is_some(),
            // TODO(cohae): Double check this
            WideHash::Hash64(h) => h.0 != 0 && h.0 != u64::MAX,
        }
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }
}

impl Debug for WideHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WideHash::Hash32(h) => f.write_fmt(format_args!("Hash32({h})")),
            WideHash::Hash64(h) => f.write_fmt(format_args!("Hash64({h})")),
        }
    }
}

impl Display for WideHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WideHash::Hash32(h) => <TagHash as Display>::fmt(h, f),
            WideHash::Hash64(h) => <TagHash64 as Display>::fmt(h, f),
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

impl From<TagHash64> for WideHash {
    fn from(val: TagHash64) -> Self {
        WideHash::Hash64(val)
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
            WideHash::Hash32(h) => Ok(WideTag(
                package_manager()
                    .read_tag_struct(h)
                    .map_err(|e| tiger_parse::Error::TagReadFailed(e.to_string()))?,
                h,
            )),
            WideHash::Hash64(h) => Ok(WideTag(
                package_manager()
                    .read_tag64_struct(h)
                    .map_err(|e| tiger_parse::Error::TagReadFailed(e.to_string()))?,
                tag.hash32(),
            )),
        }
    }

    const SIZE: usize = TagHash::SIZE;
}

impl<T: TigerReadable> std::ops::Deref for WideTag<T> {
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
