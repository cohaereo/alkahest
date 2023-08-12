use crate::structure::{DeadBeefMarker, TablePointer};
use crate::types::{Vector2, Vector4};

use binrw::BinRead;

use destiny_pkg::TagHash;

use std::cmp::Ordering;
use std::io::SeekFrom;

#[derive(BinRead, Debug)]
pub struct Unk808073a5 {
    pub file_size: u64,
    pub unk0: u64,
    pub unk1: TablePointer<Unk80807378>,
    #[br(seek_before(SeekFrom::Current(0x30)))]
    pub model_scale: Vector4,
    pub model_translation: Vector4,
    pub texcoord_scale: Vector2,
    pub texcoord_translation: Vector2,
}

#[derive(BinRead, Debug)]
pub struct Unk80807378 {
    pub position_buffer: TagHash,
    pub secondary_vertex_buffer: TagHash,
    pub buffer2: TagHash,
    pub buffer3: TagHash,
    pub index_buffer: TagHash,
    pub unk0: u32,
    pub unk1: TablePointer<Unk8080737e>,
    pub unk2: [u16; 48],
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk8080737e {
    pub material: TagHash,
    pub unk4: u16,
    pub primitive_type: EPrimitiveType,
    pub unk7: u8,
    pub index_start: u32,
    pub index_count: u32,
    pub unk10: u32,
    pub unk14: u32,
    pub unk18: u8,
    pub unk19: u8,
    pub unk1a: u8,
    pub lod_category: ELodCategory,
    pub unk1c: u32,
}

#[derive(BinRead, Debug, PartialEq, Copy, Clone)]
#[br(repr(u8))]
pub enum EPrimitiveType {
    Triangles = 3,
    TriangleStrip = 5,
}

#[allow(non_camel_case_types, clippy::derive_ord_xor_partial_ord)]
#[derive(BinRead, Debug, PartialEq, Eq, Ord, Copy, Clone)]
#[br(repr(u8))]
pub enum ELodCategory {
    /// main geometry lod0
    Lod_0_0 = 0,
    /// grip/stock lod0
    Lod_0_1 = 1,
    /// stickers lod0
    Lod_0_2 = 2,
    /// internal geom lod0
    Lod_0_3 = 3,
    /// low poly geom lod1
    Lod_1_0 = 4,
    /// low poly geom lod2
    Lod_2_0 = 7,
    /// grip/stock/scope lod2
    Lod_2_1 = 8,
    /// low poly geom lod3
    Lod_3_0 = 9,
    /// detail lod0
    Lod_Detail = 10,
}

impl PartialOrd for ELodCategory {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.remap_order().partial_cmp(&other.remap_order())
    }
}

impl ELodCategory {
    // Remap the order of variants for sorting purposes, starting with the lowest level
    fn remap_order(&self) -> u8 {
        match self {
            ELodCategory::Lod_Detail => 10,
            ELodCategory::Lod_0_0 => 9,
            ELodCategory::Lod_0_1 => 8,
            ELodCategory::Lod_0_2 => 7,
            ELodCategory::Lod_0_3 => 4,
            ELodCategory::Lod_1_0 => 3,
            ELodCategory::Lod_2_0 => 2,
            ELodCategory::Lod_2_1 => 1,
            ELodCategory::Lod_3_0 => 0,
        }
    }

    pub fn is_highest_detail(&self) -> bool {
        matches!(
            self,
            ELodCategory::Lod_0_0
                | ELodCategory::Lod_0_1
                | ELodCategory::Lod_0_2
                | ELodCategory::Lod_0_3
                | ELodCategory::Lod_Detail
        )
    }
}

#[derive(BinRead, Debug)]
pub struct VertexBufferHeader {
    pub data_size: u32,
    pub stride: u16,
    pub vtype: u16,
    pub deadbeef: DeadBeefMarker,
}

#[derive(BinRead, Debug)]
pub struct IndexBufferHeader {
    pub unk0: i8,
    #[br(map(| v: u8 | v != 0))]
    pub is_32bit: bool,
    // Probably padding
    pub unk1: u16,
    pub zero: u32,
    pub data_size: u64,
    pub deadbeef: DeadBeefMarker,
    pub zero1: u32,
}
