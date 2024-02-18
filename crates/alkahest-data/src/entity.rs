use destiny_pkg::TagHash;
use tiger_parse::{tiger_tag, FnvHash, NullString, Pointer, PointerOptional, ResourcePointer};

use super::geometry::{ELodCategory, EPrimitiveType};

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80809c0f {
    pub file_size: u64,
    pub entity_resources: Vec<Unk80809c04>,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80809ACD)]
pub struct Unk80809c04 {
    pub unk0: super::Tag<Unk80809b06>,
    pub unk4: u32,
    pub unk8: u32,
}

/// Entity resource
#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff, size = 0x90)]
pub struct Unk80809b06 {
    pub file_size: u64,
    pub unk8: ResourcePointer,
    pub unk10: ResourcePointer,
    pub unk18: ResourcePointer,

    #[tag(offset = 0x80)]
    pub unk80: TagHash,
    pub unk84: TagHash,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff, size = 0x70)]
pub struct SEntityModel {
    pub file_size: u64,
    pub unk8: u64,
    pub meshes: Vec<SEntityModelMesh>,
    #[tag(offset = 0x50)]
    pub model_scale: glam::Vec4,
    pub model_offset: glam::Vec4,
    pub texcoord_scale: glam::Vec2,
    pub texcoord_offset: glam::Vec2,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806EC5)]
pub struct SEntityModelMesh {
    pub vertex_buffer1: TagHash,
    pub vertex_buffer2: TagHash,
    pub buffer2: TagHash,
    pub buffer3: TagHash,
    pub index_buffer: TagHash,
    pub color_buffer: TagHash,
    pub skinning_buffer: TagHash,
    pub unk1c: u32,
    pub parts: Vec<Unk8080737e>,
    pub unk30: [u16; 37],
    _pad7a: [u16; 3],
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806ECB)]
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

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806D97)]
pub struct Unk808072c5 {
    pub material_count: u32,
    pub material_start: u32,
    pub unk8: u32,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff)]
pub struct VertexBufferHeader {
    pub data_size: u32,
    pub stride: u16,
    pub vtype: u16,
    // pub deadbeef: DeadBeefMarker,
    pub deadbeef: u32,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff)]
pub struct IndexBufferHeader {
    pub unk0: i8,
    pub is_32bit: bool,
    // Probably padding
    pub unk1: u16,
    pub zero: u32,
    pub data_size: u64,
    // pub deadbeef: DeadBeefMarker,
    pub deadbeef: u32,
    pub zero1: u32,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80809905 {
    pub name_hash: FnvHash,
    _pad: u32,
    pub world_id: u64,
}

#[derive(Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk8080906b {
    pub file_size: u64,
    pub unk0: Vec<Unk80809d02>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x80809D02)]
pub struct Unk80809d02 {
    pub unk0_name_pointer: PointerOptional<Unk8080894d>,
    pub unk8: PointerOptional<()>,
}

#[derive(Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk8080894d {
    pub name: Pointer<NullString>,
}
