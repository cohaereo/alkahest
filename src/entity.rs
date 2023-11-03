use crate::structure::{DeadBeefMarker, RelPointer, ResourcePointer, TablePointer, Tag};
use crate::types::{FnvHash, ResourceHash, Vector2, Vector4};

use binrw::{BinRead, BinReaderExt, NullString};

use destiny_pkg::TagHash;
use windows::Win32::Graphics::Direct3D::{
    D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
    D3D_PRIMITIVE_TOPOLOGY,
};

use std::cmp::Ordering;
use std::io::SeekFrom;

#[derive(BinRead, Debug)]
pub struct Unk80809c0f {
    pub file_size: u64,
    #[br(seek_before(SeekFrom::Start(0x8)))]
    pub entity_resources: TablePointer<Unk80809c04>,
}

#[derive(BinRead, Debug)]
pub struct Unk80809c04 {
    pub unk0: Tag<Unk80809b06>,
    pub unk4: u32,
    pub unk8: u32,
}

/// Entity resource
#[derive(BinRead, Debug)]
pub struct Unk80809b06 {
    pub file_size: u64,
    pub unk8: ResourcePointer,
    pub unk10: ResourcePointer,
    pub unk18: ResourcePointer,

    #[br(seek_before(SeekFrom::Start(0x80)))]
    pub unk80: TagHash,
    pub unk84: TagHash,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk808073a5 {
    pub file_size: u64,
    pub unk8: u64,
    pub meshes: TablePointer<Unk80807378>,
    #[br(seek_before(SeekFrom::Start(0x50)))]
    pub model_scale: Vector4,
    pub model_offset: Vector4,
    pub texcoord_scale: Vector2,
    pub texcoord_offset: Vector2,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80807378 {
    pub vertex_buffer1: TagHash,
    pub vertex_buffer2: TagHash,
    pub buffer2: TagHash,
    pub buffer3: TagHash,
    pub index_buffer: TagHash,
    pub color_buffer: TagHash,
    pub skinning_buffer: TagHash,
    pub unk1c: u32,
    pub parts: TablePointer<Unk8080737e>,
    pub unk30: [u16; 37],
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk8080737e {
    pub material: TagHash,
    pub variant_shader_index: u16,
    pub primitive_type: EPrimitiveType,
    pub unk7: u8,
    pub index_start: u32,
    pub index_count: u32,
    pub unk10: u32,
    pub external_identifier: u16,
    pub unk16: u16,
    pub flags: u32,
    pub gear_dye_change_color_index: u8,
    pub lod_category: ELodCategory,
    pub unk1e: u8,
    pub lod_run: u8,
    pub unk20: u32,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk808072c5 {
    pub material_count: u32,
    pub material_start: u32,
    pub unk6: u32,
}

#[derive(BinRead, Debug, PartialEq, Copy, Clone)]
#[br(repr(u8))]
pub enum EPrimitiveType {
    Triangles = 3,
    TriangleStrip = 5,
}

impl EPrimitiveType {
    pub fn to_dx(self) -> D3D_PRIMITIVE_TOPOLOGY {
        match self {
            EPrimitiveType::Triangles => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            EPrimitiveType::TriangleStrip => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
        }
    }
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
        Some(self.remap_order().cmp(&other.remap_order()))
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

#[derive(BinRead, Debug)]
pub struct Unk80809905 {
    pub name_hash: FnvHash,
    _pad: u32,
    pub world_id: u64,
}

#[derive(BinRead, Debug)]
pub struct Unk8080906b {
    pub file_size: u64,
    pub unk0: TablePointer<Unk80809d02>,
}

#[derive(Debug)]
pub struct Unk80809d02 {
    pub unk0_name_pointer: Option<RelPointer<Unk8080894d>>,
    pub unk8: Option<RelPointer<()>>,
}

// TODO: Optional relpointers
impl BinRead for Unk80809d02 {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let check: [u64; 2] = reader.read_type(endian)?;
        reader.seek(SeekFrom::Current(-16))?;

        let unk0_name_pointer = if check[0] != 0 {
            reader.read_type(endian)?
        } else {
            reader.seek(SeekFrom::Current(8))?;
            None
        };

        let unk8 = if check[1] != 0 {
            reader.read_type(endian)?
        } else {
            reader.seek(SeekFrom::Current(8))?;
            None
        };

        Ok(Self {
            unk0_name_pointer,
            unk8,
        })
    }
}

#[derive(BinRead, Debug)]
pub struct Unk8080894d {
    pub name: RelPointer<NullString>,
}
