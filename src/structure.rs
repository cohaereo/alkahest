use binrw::{BinRead, BinReaderExt, BinResult, Endian};
use destiny_pkg::TagHash;
use tiger_parse::TigerReadable;

use std::fmt::{Debug, Formatter, Write};
use std::io::{Read, Seek, SeekFrom};
use std::ops::Deref;
use std::slice::Iter;

use crate::packages::package_manager;

// pub type TablePointer32<T> = _TablePointer<i32, u32, T>;
pub type TablePointer64<T> = _TablePointer<i64, u64, T>;
pub type TablePointer<T> = TablePointer64<T>;

// pub type RelPointer32<T = ()> = _RelPointer<i32, T>;
pub type RelPointer64<T = ()> = _RelPointer<i64, T>;
pub type RelPointer<T = ()> = RelPointer64<T>;

#[derive(Clone)]
pub struct _TablePointer<O: Into<i64>, C: Into<u64>, T: BinRead> {
    offset_base: u64,
    offset: O,
    count: C,

    data: Vec<T>,
}

impl<'a, O: Into<i64>, C: Into<u64>, T: BinRead> BinRead for _TablePointer<O, C, T>
where
    C: BinRead + Copy,
    O: BinRead + Copy,
    C::Args<'a>: Default + Clone,
    O::Args<'a>: Default + Clone,
    T::Args<'a>: Default + Clone,
{
    type Args<'b> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<Self> {
        let count: C = reader.read_type(endian)?;
        let offset_base = reader.stream_position()?;
        let offset: O = reader.read_type(endian)?;

        let offset_save = reader.stream_position()?;

        let seek64: i64 = offset.into();
        reader.seek(SeekFrom::Start(offset_base))?;
        reader.seek(SeekFrom::Current(seek64))?;
        // TODO(cohae): Check array header
        reader.seek(SeekFrom::Current(16))?;

        let count64: u64 = count.into();
        let mut data = Vec::with_capacity(count64 as _);
        for _ in 0..count64 {
            data.push(reader.read_type(endian)?);
        }

        reader.seek(SeekFrom::Start(offset_save))?;

        Ok(_TablePointer {
            offset_base,
            offset,
            count,
            data,
        })
    }
}

impl<O: Into<i64> + Copy, C: Into<u64> + Copy, T: BinRead> _TablePointer<O, C, T> {
    pub fn iter(&self) -> Iter<'_, T> {
        self.data.iter()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn data(&self) -> &[T] {
        &self.data
    }

    pub fn take_data(self) -> Vec<T> {
        self.data
    }
}

impl<O: Into<i64> + Copy, C: Into<u64> + Copy, T: BinRead> Deref for _TablePointer<O, C, T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a, O: Into<i64> + Copy, C: Into<u64> + Copy, T: BinRead> IntoIterator
    for &'a _TablePointer<O, C, T>
{
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<O: Into<i64> + Copy, C: Into<u64> + Copy, T: BinRead + Debug> Debug
    for _TablePointer<O, C, T>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "TablePointer(address=0x{:x}, count={}",
            self.offset_base as i64 + self.offset.into(),
            self.count.into(),
        ))?;

        f.write_str(", data=")?;
        self.data.fmt(f)?;

        f.write_char(')')
    }
}

#[derive(Clone, Copy)]
pub struct _RelPointer<O: Into<i64>, T: BinRead> {
    offset_base: u64,
    offset: O,

    data: T,
}

impl<'a, O: Into<i64>, T: BinRead> BinRead for _RelPointer<O, T>
where
    O: BinRead + Copy,
    O::Args<'a>: Default + Clone,
    T::Args<'a>: Default + Clone,
{
    type Args<'b> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<Self> {
        let offset_base = reader.stream_position()?;
        let offset: O = reader.read_type(endian)?;

        let offset_save = reader.stream_position()?;

        let seek64: i64 = offset.into();
        reader.seek(SeekFrom::Start(offset_base))?;
        reader.seek(SeekFrom::Current(seek64))?;

        let data = reader.read_type(endian)?;

        reader.seek(SeekFrom::Start(offset_save))?;

        Ok(_RelPointer {
            offset_base,
            offset,
            data,
        })
    }
}

impl<O: Into<i64> + Copy, T: BinRead> Deref for _RelPointer<O, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<O: Into<i64> + Copy, T: BinRead + Debug> Debug for _RelPointer<O, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "RelPointer(address=0x{:x}",
            self.offset_base as i64 + self.offset.into(),
        ))?;

        f.write_str(", data=")?;
        self.data.fmt(f)?;

        f.write_char(')')
    }
}

impl<O: Into<i64> + Copy, T: BinRead + Debug> From<_RelPointer<O, T>> for SeekFrom {
    fn from(val: _RelPointer<O, T>) -> Self {
        SeekFrom::Start((val.offset_base as i64 + val.offset.into()) as u64)
    }
}

#[derive(BinRead)]
pub struct CafeMarker(#[br(assert(self_0 == 0xcafe))] u16);

#[derive(BinRead)]
pub struct DeadBeefMarker(#[br(assert(self_0 == 0xdeadbeef))] u32);

impl Debug for CafeMarker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("CafeMarker")
    }
}

impl Debug for DeadBeefMarker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeadBeefMarker")
    }
}

#[derive(Clone, Copy)]
pub struct ResourcePointer {
    pub offset: u64,
    pub resource_type: u32,
    pub is_valid: bool,
}

impl BinRead for ResourcePointer {
    type Args<'b> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<Self> {
        let offset_base = reader.stream_position()?;
        let offset: i64 = reader.read_type(endian)?;
        if offset == 0 || offset == i64::MAX {
            return Ok(ResourcePointer {
                offset: 0,
                resource_type: u32::MAX,
                is_valid: false,
            });
        }

        let offset_save = reader.stream_position()?;

        reader.seek(SeekFrom::Start(offset_base))?;
        reader.seek(SeekFrom::Current(offset - 4))?;
        let resource_type: u32 = reader.read_type(endian)?;

        reader.seek(SeekFrom::Start(offset_save))?;

        Ok(ResourcePointer {
            offset: offset_base.saturating_add_signed(offset),
            resource_type,
            is_valid: true,
        })
    }
}

impl TigerReadable for ResourcePointer {
    fn read_ds_endian<R: Read + Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> anyhow::Result<Self> {
        let offset_base = reader.stream_position()?;
        let offset: i64 = TigerReadable::read_ds_endian(reader, endian)?;
        if offset == 0 || offset == i64::MAX {
            return Ok(ResourcePointer {
                offset: 0,
                resource_type: u32::MAX,
                is_valid: false,
            });
        }

        let offset_save = reader.stream_position()?;

        reader.seek(SeekFrom::Start(offset_base))?;
        reader.seek(SeekFrom::Current(offset - 4))?;
        let resource_type: u32 = TigerReadable::read_ds_endian(reader, endian)?;

        reader.seek(SeekFrom::Start(offset_save))?;

        Ok(ResourcePointer {
            offset: offset_base.saturating_add_signed(offset),
            resource_type,
            is_valid: true,
        })
    }

    const ID: Option<u32> = None;
    const ZEROCOPY: bool = false;
    const SIZE: usize = 8;
}

impl Debug for ResourcePointer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "ResourcePointer(type=0x{:08x})",
            self.resource_type
        ))
    }
}

#[derive(Clone, Copy)]
pub struct ResourcePointerWithClass {
    pub offset: u64,
    pub is_valid: bool,

    pub resource_type: u32,
    /// Usually just the current tag
    pub parent_tag: TagHash,
    pub class_type: u32,
}

impl BinRead for ResourcePointerWithClass {
    type Args<'b> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<Self> {
        let offset_base = reader.stream_position()?;
        let offset: i64 = reader.read_type(endian)?;
        if offset == 0 || offset == i64::MAX {
            return Ok(ResourcePointerWithClass {
                offset: 0,
                is_valid: false,
                resource_type: u32::MAX,
                parent_tag: TagHash::NONE,
                class_type: u32::MAX,
            });
        }

        let offset_save = reader.stream_position()?;

        reader.seek(SeekFrom::Start(offset_base))?;
        reader.seek(SeekFrom::Current(offset - 4))?;
        let resource_type: u32 = reader.read_type(endian)?;
        let parent_tag: TagHash = reader.read_type(endian)?;
        let class_type: u32 = reader.read_type(endian)?;

        let true_offset = reader.stream_position()?;
        reader.seek(SeekFrom::Start(offset_save))?;

        Ok(ResourcePointerWithClass {
            offset: true_offset,
            is_valid: true,
            resource_type,
            parent_tag,
            class_type,
        })
    }
}

impl TigerReadable for ResourcePointerWithClass {
    fn read_ds_endian<R: Read + Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> anyhow::Result<Self> {
        let offset_base = reader.stream_position()?;
        let offset: i64 = TigerReadable::read_ds_endian(reader, endian)?;
        if offset == 0 || offset == i64::MAX {
            return Ok(ResourcePointerWithClass {
                offset: 0,
                is_valid: false,
                resource_type: u32::MAX,
                parent_tag: TagHash::NONE,
                class_type: u32::MAX,
            });
        }

        let offset_save = reader.stream_position()?;

        reader.seek(SeekFrom::Start(offset_base))?;
        reader.seek(SeekFrom::Current(offset - 4))?;
        let resource_type: u32 = TigerReadable::read_ds_endian(reader, endian)?;
        let parent_tag: TagHash = TigerReadable::read_ds_endian(reader, endian)?;
        let class_type: u32 = TigerReadable::read_ds_endian(reader, endian)?;

        let true_offset = reader.stream_position()?;
        reader.seek(SeekFrom::Start(offset_save))?;

        Ok(ResourcePointerWithClass {
            offset: true_offset,
            is_valid: true,
            resource_type,
            parent_tag,
            class_type,
        })
    }

    const ID: Option<u32> = None;
    const ZEROCOPY: bool = false;
    const SIZE: usize = 8;
}

impl Debug for ResourcePointerWithClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "ResourcePointer(type=0x{:08X}, parent={}, class={:08X})",
            self.resource_type,
            self.parent_tag,
            self.class_type.to_be()
        ))
    }
}

#[derive(Clone, Copy)]
pub struct Tag<T: BinRead>(pub T, TagHash);

impl<T: BinRead> Tag<T> {
    pub fn tag(&self) -> TagHash {
        self.1
    }
}

impl<'a, T: BinRead> BinRead for Tag<T>
where
    T::Args<'a>: Default + Clone,
{
    type Args<'b> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<Self> {
        let taghash: TagHash = reader.read_type(endian)?;
        Ok(Tag(
            package_manager()
                .read_tag_binrw(taghash)
                .map_err(|e| binrw::Error::Custom {
                    pos: reader.stream_position().unwrap(),
                    err: Box::new(e),
                })?,
            taghash,
        ))
    }
}

impl<T: BinRead> Deref for Tag<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: BinRead + Debug> Debug for Tag<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
