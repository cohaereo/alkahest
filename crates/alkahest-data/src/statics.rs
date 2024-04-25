use destiny_pkg::TagHash;
use tiger_parse::tiger_tag;

use crate::{
    geometry::{ELodCategory, EPrimitiveType},
    occlusion::SOcclusionBounds,
    tag::Tag,
    tfx::TfxRenderStage,
};

#[derive(Debug)]
#[tiger_tag(id = 0x80806D44)]
pub struct SStaticMesh {
    pub file_size: u64,
    /// GenerateGbuffer/DepthPrepass/ShadowGenerate
    pub opaque_meshes: Tag<SStaticMeshData>,
    pub unkc: u32,
    pub techniques: Vec<TagHash>,
    /// Transparents, decals, light shaft occluders, etc.
    pub special_meshes: Vec<SStaticSpecialMesh>,
    pub unk30: [u32; 2],
    pub unk38: [f32; 6],
    pub unk50: glam::Vec4, // ? Similar to model_offset, but not quite right...
    pub unk60: glam::Vec4,
}

#[derive(Debug)]
#[tiger_tag(id = 0x80806D30, size = 0x60)]
pub struct SStaticMeshData {
    pub file_size: u64,
    pub mesh_groups: Vec<SStaticMeshGroup>,
    pub parts: Vec<SStaticMeshPart>,
    pub buffers: Vec<(TagHash, TagHash, TagHash, TagHash)>,

    #[tag(offset = 0x40)]
    pub mesh_offset: glam::Vec3,
    pub mesh_scale: f32,
    pub texture_coordinate_scale: f32,
    pub texture_coordinate_offset: glam::Vec2,
    pub max_color_index: u32,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806D37)]
pub struct SStaticMeshPart {
    pub index_start: u32,
    pub index_count: u32,
    pub buffer_index: u8,
    pub unk9: u8,
    pub lod_category: ELodCategory,
    pub primitive_type: EPrimitiveType,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806D38)]
pub struct SStaticMeshGroup {
    pub part_index: u16,
    pub render_stage: TfxRenderStage,
    pub input_layout_index: u8,
    pub unk5: u16,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x808093AD, size = 0x98)]
pub struct SStaticMeshInstances {
    #[tag(offset = 0x18)]
    pub occlusion_bounds: Tag<SOcclusionBounds>,

    #[tag(offset = 0x40)]
    pub transforms: Vec<SStaticInstanceTransform>,
    pub unk50: u64,
    pub unk58: [u64; 4],
    pub statics: Vec<TagHash>,
    pub instance_groups: Vec<SStaticMeshInstanceGroup>,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806D28)]
pub struct SStaticMeshInstanceGroup {
    pub instance_count: u16,
    pub instance_start: u16,
    pub static_index: u16,
    pub unk6: u16,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806D40)]
pub struct SStaticInstanceTransform {
    pub rotation: glam::Quat,
    pub translation: glam::Vec3,
    pub scale: glam::Vec3,
    pub unk28: u32,
    pub unk2c: u32,
    pub unk30: [u32; 4],
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806D2F)]
pub struct SStaticSpecialMesh {
    pub render_stage: TfxRenderStage,
    pub input_layout_index: u8,
    pub lod: ELodCategory,
    pub unk3: i8,
    pub primitive_type: EPrimitiveType,
    pub unk5: u8,
    pub unk6: u16,
    pub index_buffer: TagHash,
    pub vertex0_buffer: TagHash,
    pub vertex1_buffer: TagHash,
    pub color_buffer: TagHash,
    pub index_start: u32,
    pub index_count: u32,
    pub technique: TagHash,
}
