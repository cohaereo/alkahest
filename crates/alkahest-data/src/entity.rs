use std::ops::Range;

use destiny_pkg::TagHash;
use tiger_parse::{tiger_tag, FnvHash, NullString, Pointer, PointerOptional, ResourcePointer};

use super::geometry::{ELodCategory, EPrimitiveType};
use crate::{activity::SEntityResource, tfx::TfxRenderStage, Tag};

#[derive(Clone)]
#[tiger_tag(id = 0x80809AD8)]
pub struct SEntity {
    pub file_size: u64,
    pub entity_resources: Vec<Unk80809c04>,
}

#[derive(Clone)]
#[tiger_tag(id = 0x80809ACD)]
pub struct Unk80809c04 {
    pub unk0: Tag<SEntityResource>,
    pub unk4: u32,
    pub unk8: u32,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806F07, size = 0x70)]
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
#[tiger_tag(id = 0x80806EC5, size = 0x80)]
pub struct SDynamicMesh {
    pub vertex0_buffer: TagHash,
    pub vertex1_buffer: TagHash,
    pub buffer2: TagHash,
    pub buffer3: TagHash,
    pub index_buffer: TagHash,
    pub color_buffer: TagHash,
    pub skinning_buffer: TagHash,
    pub unk1c: u32,
    pub parts: Vec<SDynamicMeshPart>, // 0x20
    /// Range of parts to render per render stage
    /// Can be obtained as follows:
    ///
    ///     - Start = part_range_per_render_stage[stage]
    ///     - End = part_range_per_render_stage[stage + 1]
    pub part_range_per_render_stage: [u16; 25], // 0x30
    pub input_layout_per_render_stage: [u8; 24], // 0x62
    _pad7a: [u16; 3],
}

impl SDynamicMesh {
    pub fn get_range_for_stage(&self, stage: TfxRenderStage) -> Range<usize> {
        let start = self.part_range_per_render_stage[stage as usize];
        let end = self.part_range_per_render_stage[stage as usize + 1];
        start as usize..end as usize
    }

    pub fn get_input_layout_for_stage(&self, stage: TfxRenderStage) -> u8 {
        self.input_layout_per_render_stage[stage as usize]
    }
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806ECB)]
pub struct SDynamicMeshPart {
    pub technique: TagHash,
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
