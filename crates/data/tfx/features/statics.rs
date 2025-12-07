use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

use crate::{
    tag::Tag,
    tfx::{
        common::{AxisAlignedBBox, SOcclusionBounds},
        LodCategory, PrimitiveType, RenderStage,
    },
};

#[derive(Debug)]
#[tiger_type(id = 0x80808635)]
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
    pub unk50: [u32; 4],
    pub unk60: [u32; 4],
}

#[derive(Debug)]
#[tiger_type(id = 0x80808620, size = 0x60)]
pub struct SStaticMeshData {
    pub file_size: u64,
    pub mesh_groups: Vec<SStaticMeshGroup>,
    pub parts: Vec<SStaticMeshPart>,
    pub buffers: Vec<(TagHash, TagHash, TagHash, TagHash)>,
    pub unk38: u32,

    #[tiger(offset = 0x40)]
    pub mesh_offset: glam::Vec3,
    pub mesh_scale: f32,
    pub texture_coordinate_scale: f32,
    pub texture_coordinate_offset: glam::Vec2,
    pub max_color_index: u32,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x80808627)]
pub struct SStaticMeshPart {
    pub index_start: u32,
    pub index_count: u32,
    pub buffer_index: u8,
    pub unk9: u8,
    pub lod_category: LodCategory,
    pub primitive_type: PrimitiveType,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x80808628)]
pub struct SStaticMeshGroup {
    pub part_index: u16,
    pub render_stage: RenderStage,
    pub input_layout_index: u8,
    pub unk5: u8,
    /// Usually 1.
    /// If 2, at least for render_stage=ShadowGenerate, the geometry in this group has some kind of vertex animation
    /// This can be used to differentiate stationary static geometry from moving/animated statics
    pub unk6: u8,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x8080A7F1, size = 0xC0)]
pub struct SStaticMeshInstances {
    #[tiger(offset = 0x18)]
    pub occlusion_bounds: Tag<SOcclusionBounds>,
    #[tiger(offset = 0x40)]
    pub transforms: Vec<SStaticInstanceTransform>,
    pub unk50: u64,
    pub unk58: [u64; 4],
    pub statics: Vec<TagHash>,
    pub instance_groups: Vec<SStaticMeshInstanceGroup>,
    pub vertex_ao_identifier: u64,
    pub bounds: AxisAlignedBBox,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x80808618)]
pub struct SStaticMeshInstanceGroup {
    pub instance_start: u32,
    pub instance_count: u32,
    pub static_index: u32,
    pub unk6: u32,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x8080862F)]
pub struct SStaticInstanceTransform {
    pub rotation: glam::Quat,
    pub translation: glam::Vec3,
    pub scale: f32,

    pub unk20: [u32; 4],
    pub unk30: [u32; 4],
    pub unk40: [u32; 4],
    pub unk50: [u32; 4],
    // pub unk28: u32,
    // pub unk2c: u32,
    // pub unk30: [u32; 4],
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x8080861F)]
pub struct SStaticSpecialMesh {
    pub render_stage: RenderStage,
    pub input_layout_index: u8,
    pub lod: LodCategory,
    pub primitive_type: PrimitiveType,

    // 0x4
    pub index_buffer: TagHash,
    pub vertex0_buffer: TagHash,
    pub vertex1_buffer: TagHash,
    // 0x10
    pub color_buffer: TagHash,
    pub index_start: u32,
    pub index_count: u32,
    pub technique: TagHash,
}

#[derive(Debug)]
#[tiger_type(id = 0x808082D5, size = 0x24)]
pub struct SUnk808082D5 {
    pub unk0: u64,
    pub instances: TagHash, //Tag<SStaticMeshInstances>,
}
