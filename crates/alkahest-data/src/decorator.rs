use destiny_pkg::TagHash;
use glam::Vec4;
use tiger_parse::tiger_tag;

use crate::{
    occlusion::{SOcclusionBounds, AABB},
    Tag,
};

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806C98)]
pub struct SDecorator {
    pub file_size: u64,
    pub unk8: Vec<Tag<SUnk80806CB2>>,
    pub unk18: Vec<u32>,
    pub unk28: Vec<u32>,
    pub unk38: Vec<u32>,
    pub unk48: Tag<SUnk80806CA4>,
    pub unk4c: Tag<SOcclusionBounds>,
    pub unk50: Vec<u32>,
    pub unk60: [u32; 4],
    pub bounds: AABB,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806CA4)]
pub struct SUnk80806CA4 {
    pub file_size: u64,
    pub unk8: u32,
    pub unkc: u32,
    pub unk10: u32,
    pub unk14: Tag<SUnk80806C9F>,
    pub instance_buffer: TagHash,
    pub instance_data: Tag<SDecoratorInstanceData>,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806CA7)]
pub struct SDecoratorInstanceData {
    pub file_size: u64,
    pub data: Vec<SDecoratorInstanceElement>,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806CA9)]
pub struct SDecoratorInstanceElement {
    /// Normalized position
    pub position: [u16; 3],
    /// Rotation represented as an 8-bit quaternion
    pub rotation: [u8; 4],
    /// RGBA color
    pub color: [u8; 4],
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806C9F)]
pub struct SUnk80806C9F {
    pub instances_scale: Vec4,
    pub instances_offset: Vec4,
    pub unk20: Vec4,
    pub unk30: Vec4,
    pub unk40: Vec4,
    pub unk50: Vec4,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806CB2)]
pub struct SUnk80806CB2 {
    pub file_size: u64,
    pub entity_model: TagHash,
    // ...
}
