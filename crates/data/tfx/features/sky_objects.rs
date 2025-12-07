use glam::Mat4;
use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

use crate::{tag::Tag, tfx::common::AxisAlignedBBox};

#[derive(Clone, Debug)]
#[tiger_type(id = 0x8080837C)]
pub struct SSkyObjectCollection {
    pub file_size: u64,
    pub unk8: Vec<SUnk80806f97>,
    pub unk18: Vec<()>, //Vec<SObjectOcclusionBounds>,
    pub unk28: Vec<u32>,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x8080837E)]
pub struct SUnk80806f97 {
    /// Transformation matrix
    pub transform: Mat4,

    /// Same as the bounding box from the SObjectOcclusionBounds array
    pub bounds: AxisAlignedBBox,

    pub model_ref: Tag<SSkyObjectModelRef>,
    pub unk64: f32,
    pub unk68: u32,
    pub unk6c: u8,
    pub unk6d: u8,
    pub unk6e: u8,
    pub unk6f: u8,

    pub unk70: u32,
    pub unk74: f32,
    pub unk78: u32,
    pub unk7c: TagHash,

    pub unk80: u64,
    pub unk88: u32,
    pub unk8c: u32,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80808383)]
pub struct SSkyObjectModelRef {
    pub file_size: u64,
    pub entity_model: TagHash,
}
