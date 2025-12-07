use glam::{Mat4, Vec4};
use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

use crate::{
    tag::Tag,
    tfx::common::{AxisAlignedBBox, SOcclusionBounds, SRotationTranslation},
};

#[derive(Clone, Debug)]
#[tiger_type(id = 0x8080854B)]
pub struct SLightCollection {
    pub file_size: u64,
    pub unk8: u64,
    pub bounds: AxisAlignedBBox,
    pub lights: Vec<SLight>,
    pub transforms: Vec<SRotationTranslation>,
    pub light_count: u32,
    pub unk54: u32,
    pub occlusion_bounds: Tag<SOcclusionBounds>,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80808556, size = 256)]
pub struct SLight {
    pub unk0: Vec4,
    pub unk10: Vec4,
    pub unk20: Vec4,
    pub unk30: Vec4,
    pub unk40: [u32; 4],
    pub unk50: Vec4,
    pub light_space_transform: Mat4,
    pub unka0: u32,
    pub unka4: u32,
    pub unka8: u32,
    pub unkac: f32,
    pub unkb0: f32,
    pub unkb4: f32,
    pub unkb8: f32,
    pub unkbc: f32,

    pub technique_lighting_apply: TagHash,
    pub technique_volumetrics: TagHash,
    pub technique_lightprobe_apply_lightprobe: TagHash,
    pub unkc8: TagHash, // Unk80806da1
    pub unkd0: TagHash, // Unk80806da1
    pub unkd4: [u32; 7],
}

#[tiger_type(id = 0x80808557, size = 0x120)]
pub struct SShadowingLight {
    pub unk0: Vec4,
    pub unk10: Vec4,
    pub unk20: Vec4,
    pub unk30: Vec4,
    pub unk40: [u32; 4],
    pub unk50: Vec4,
    pub light_space_transform: Mat4,
    pub unka0: u32,
    pub unka4: u32,
    pub unka8: u32,
    pub unkac: f32,
    pub unkb0: f32,
    pub unkb4: f32,
    pub unkb8: f32,

    pub unkbc: f32,

    pub far_plane: f32,
    pub half_fov: f32,

    pub unkc8: u32,
    pub unkcc: f32,

    // 0xD0
    pub technique_lighting_apply: TagHash,
    pub technique_lighting_apply_shadowing: TagHash,
    pub technique_volumetrics: TagHash,
    pub technique_volumetrics_shadowing: TagHash,
    // 0xE0
    pub technique_lightprobe_apply_lightprobe: TagHash,
    pub technique_lightprobe_apply_lightprobe_shadowing: TagHash,

    pub unke8: TagHash, // 80808691
    pub unkec: TagHash, // 80808691

    pub unkf0: [f32; 5],
    pub unk104: [u8; 12],
}
