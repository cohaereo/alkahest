use std::ops::Range;

use destiny_pkg::TagHash;
use tiger_parse::{tiger_tag, FnvHash, NullString, Pointer, PointerOptional};

use super::geometry::{ELodCategory, EPrimitiveType};
use crate::{activity::SEntityResource, tfx::TfxRenderStage, Tag};

#[derive(Clone)]
#[tiger_tag(id = 0x80809C0F)]
pub struct SEntity {
    pub file_size: u64,
    pub unk8: [u32; 2],
    pub entity_resources: Vec<Unk80809c04>,
}

#[derive(Clone)]
#[tiger_tag(id = 0x80809C04)]
pub struct Unk80809c04 {
    pub unk0: Tag<SEntityResource>,
    pub unk4: u32,
    pub unk8: u32,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x808073A5, size = 0xa0)]
pub struct SDynamicModel {
    pub file_size: u64,
    pub unk8: u64,
    pub meshes: Vec<SDynamicMesh>,
    pub unk20: glam::Vec4,
    #[tag(offset = 0x50)]
    pub model_scale: glam::Vec4,
    pub model_offset: glam::Vec4,
    pub texcoord_scale: glam::Vec2,
    pub texcoord_offset: glam::Vec2,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80807378, size = 0x88)]
pub struct SDynamicMesh {
    pub vertex0_buffer: TagHash,      // 0x0
    pub vertex1_buffer: TagHash,      // 0x4
    pub buffer2: TagHash,             // 0x8
    pub buffer3: TagHash,             // 0xc
    pub index_buffer: TagHash,        // 0x10
    pub unk14: u32,                   // 0x14
    pub parts: Vec<SDynamicMeshPart>, // 0x18
    /// Range of parts to render per render stage
    /// Can be obtained as follows:
    ///     - Start = part_range_per_render_stage[stage]
    ///     - End = part_range_per_render_stage[stage + 1]
    pub part_range_per_render_stage: [u16; 24], // 0x28
    pub input_layout_per_render_stage: [u16; 23], // 0x58
    _pad86: [u8; 2],
}

impl SDynamicMesh {
    pub fn get_range_for_stage(&self, stage: TfxRenderStage) -> Range<usize> {
        let start = self.part_range_per_render_stage[stage as usize];
        let end = self.part_range_per_render_stage[stage as usize + 1];
        start as usize..end as usize
    }

    pub fn get_input_layout_for_stage(&self, stage: TfxRenderStage) -> u16 {
        self.input_layout_per_render_stage[stage as usize]
    }
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x8080737E)]
pub struct SDynamicMeshPart {
    pub technique: TagHash,             // 0x0
    pub variant_shader_index: u16,      // 0x4
    pub primitive_type: EPrimitiveType, // 0x6
    pub unk7: u8,                       // 0x7
    pub index_start: u32,               // 0x8
    pub index_count: u32,               // 0xc
    pub unk10: u32,                     // 0x10
    pub external_identifier: u16,       // 0x14
    pub unk16: u16,                     // 0x16
    pub unk18: u8,                      // 0x18
    pub unk19: u8,                      // 0x19
    pub unk1a: u8,                      // 0x1a
    pub lod_category: ELodCategory,     // 0x1b
    pub unk1c: u32,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x808072C4)]
pub struct Unk808072c5 {
    pub technique_count: u32,
    pub technique_start: u32,
    pub unk8: u32,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80809905 {
    pub name_hash: FnvHash,
    _pad: u32,
    pub world_id: u64,
}

#[derive(Debug)]
#[tiger_tag(id = 0x8080906b)]
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
