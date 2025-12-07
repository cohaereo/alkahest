use glam::Vec4;
use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

use crate::{
    tag::{OptionalTag, Tag},
    tfx::common::{AxisAlignedBBox, SOcclusionBounds},
};

#[derive(Clone, Debug)]
#[tiger_type(id = 0x8080857D)]
pub struct SDecorator {
    pub file_size: u64,
    pub unk8: Vec<Tag<SUnk8080717E>>,
    pub unk18: Vec<u32>,
    pub unk28: Vec<u32>,
    pub unk38: Vec<u32>,
    pub unk48: Tag<SUnk80807170>,
    pub unk4c: Tag<SOcclusionBounds>,
    pub unk50: Vec<u32>,
    pub unk60: [u32; 4],
    pub bounds: AxisAlignedBBox,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x8080858B)]
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
#[tiger_type(id = 0x8080858E)]
pub struct SDecoratorInstanceData {
    pub file_size: u64,
    pub data: Vec<SDecoratorInstanceElement>,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80808590)]
pub struct SDecoratorInstanceElement {
    /// Normalized position
    pub position: [u16; 3],
    /// Rotation represented as an 8-bit quaternion
    pub rotation: [u8; 4],
    /// RGBA color
    pub color: [u8; 4],
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80808586)]
pub struct SUnk8080716B {
    pub instances_scale: Vec4,
    pub instances_offset: Vec4,
    pub unk20: Vec4,
    pub unk30: Vec4,
    pub unk40: Vec4,
    pub unk50: Vec4,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80808599)]
pub struct SUnk8080717E {
    pub file_size: u64,
    pub entity_model: TagHash,
    pub unk8: u32,
    pub bounds: AxisAlignedBBox,
    pub unk10: TagHash,
    pub unk14: OptionalTag<SUnk80807184>,
    pub unk18: Vec<f32>,
    pub unk28: Vec<bool>,
    pub unk38: Vec<f32>,
    // ...
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80807184)]
pub struct SUnk80807184 {
    pub file_size: u64,
    pub unk8: Vec<SUnk80807186>,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80807186)]
pub struct SUnk80807186 {
    pub unk0: [Vec4; 5],
}
