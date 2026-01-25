use glam::Vec4;
use int_enum::IntEnum;
use tiger_parse::{tiger_type, FnvHash, NullString, Padding, Pointer, TigerReadable};
use tiger_pkg::TagHash;

use crate::tag::Tag;

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808096BF)]
pub struct S808096BF {
    pub file_size: u64,
    pub destination_name: Pointer<NullString>,
    pub unk10: Vec<S808096CE>,
    pub bubbles: Vec<S808096C3>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808096CE)]
pub struct S808096CE {
    pub unk0: u64,
    pub name: FnvHash,
    pub kind: S808096CEKind,
    pub unke: u16,
    pub unk10: u32,
    pub bubble_index: i32,
    pub unk18: u32,
    pub unk1c: FnvHash,
}

#[derive(Debug, Clone, IntEnum)]
#[repr(u16)]
pub enum S808096CEKind {
    Bubble = 0,
    Transition = 1,
    Sky = 2,
}

impl TigerReadable for S808096CEKind {
    fn read_ds_endian<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let v = u16::read_ds_endian(reader, endian)?;
        Self::try_from(v).map_err(|_| tiger_parse::Error::EnumVariantOutOfRange(v as usize))
    }

    const SIZE: usize = 2;
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808096C3)]
pub struct S808096C3 {
    pub name: FnvHash,
    pub index_into_unk10: u32,
    pub unk8: [u32; 4],
    pub transitions: Vec<S80809A4F>,
    pub dependencies: Vec<S80809A4F>,
    pub unk38: u32,
    pub unk3c: u32,
    pub unk40: [u8; 4],
    pub unk44: [u32; 7],
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x80809A4F)]
pub struct S80809A4F {
    pub unk0: u16,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x80809567, size = 0xA0)]
pub struct S80809567 {
    pub file_size: u64,
    pub unk8: u64,
    pub unk10: u64,
    pub unk18: Vec<()>,
    pub unk28: Vec<S80809588>,

    #[tiger(offset = 0x90)]
    pub unk90: Tag<S8080957B>,
    pub unk94: TagHash,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x80809588)]
pub struct S80809588 {
    pub unk0: u64,
    pub unk8: u64,
    pub unk10: u64,
    pub unk18: u64,
    pub unk20: u64,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x8080957B)]
pub struct S8080957B {
    pub file_size: u64,
    pub entries: Vec<S8080957E>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x8080957E)]
pub struct S8080957E {
    pub unk0: u16,
    pub unk2: u16,
    _pad4: Padding<4>,

    pub warnings: Vec<Pointer<NullString>>,
    _pad18: Padding<8>,

    pub v0: Vec4,
    pub v1: Vec4,
    pub v2: Vec4,
}
