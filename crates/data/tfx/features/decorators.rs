use bytemuck::{Pod, Zeroable};
use glam::Vec4;
use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

use crate::{
    tag::{OptionalTag, Tag},
    tfx::common::{AxisAlignedBBox, SOcclusionBounds},
};

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80806C98)]
pub struct SDecorator {
    pub file_size: u64,
    pub unk8: Vec<Tag<SUnk8080717E>>,
    pub unk18: Vec<u32>,
    pub unk28: Vec<u32>,
    pub unk38: Vec<u32>,
    pub unk48: Tag<SUnk80807170>,
    pub occlusion_bounds: Tag<SOcclusionBounds>,
    pub unk38_to_bounds_index: Vec<u32>,
    pub unk60: [u32; 4],
    pub bounds: AxisAlignedBBox,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80806CA4)]
pub struct SUnk80807170 {
    pub file_size: u64,
    pub unk8: u32,
    pub unkc: TagHash, // Vertex buffer data? (as opposed to a header)
    pub unk10: u32,
    pub unk14: Tag<SUnk8080716B>,
    pub instance_buffer: TagHash,
    pub instance_data: Tag<SDecoratorInstanceData>,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80806CA7)]
pub struct SDecoratorInstanceData {
    pub file_size: u64,
    pub elements: Vec<SDecoratorInstanceElement>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[tiger_type(id = 0x80806CA9, size = 0x10)]
pub struct SDecoratorInstanceElement {
    /// Normalized position
    pub position: [u16; 4],
    /// Rotation represented as an 8-bit quaternion
    pub rotation: [u8; 4],
    /// RGBA color
    pub color: [u8; 4],
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80806C9F)]
pub struct SUnk8080716B {
    pub instances_scale: Vec4,
    pub instances_offset: Vec4,
    pub unk20: Vec4,
    pub unk30: Vec4,
    pub unk40: Vec4,
    pub unk50: Vec4,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80806CB2)]
pub struct SUnk8080717E {
    pub file_size: u64,
    pub entity_model: TagHash,
    pub unk8: u32,
    pub bounds: AxisAlignedBBox,
    pub unk30: TagHash,
    pub unk34: OptionalTag<SUnk80807184>,
    pub unk38: Vec<f32>,
    pub unk48: Vec<bool>,
    pub unk58: Vec<f32>,
    // ...
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80806CB8)]
pub struct SUnk80807184 {
    pub file_size: u64,
    pub unk8: Vec<SUnk80807186>,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80806CBA)]
pub struct SUnk80807186 {
    pub unk0: [Vec4; 5],
}
