use glam::{Quat, Vec2, Vec4};
use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

use crate::{tag::Tag, tfx::common::SOcclusionBounds};

#[tiger_type(id = 0x808068EA)]
#[derive(Debug, Clone)]
pub struct SRoadDecalCollection {
    pub file_size: u64,
    pub decals: Vec<SRoadDecal>,
    pub occlusion_bounds: Tag<SOcclusionBounds>,
}

#[tiger_type(id = 0x808068E3, size = 0x60)]
#[derive(Debug, Clone)]
pub struct SRoadDecal {
    pub technique: TagHash,
    pub index_buffer: TagHash,
    pub vertex_buffer: TagHash,
    pub face_count: u16,
    pub index_start: u16,
    pub rotation: Quat,
    pub position: Vec4,

    pub model_scale: Vec4,
    pub model_offset: Vec4,
    pub texcoord_scale: Vec2,
    pub texcoord_offset: Vec2,
}
