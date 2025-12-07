use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

use crate::{
    tag::Tag,
    tfx::common::{AxisAlignedBBox, SOcclusionBounds},
};

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80808224)]
pub struct SDecalCollection {
    pub file_size: u64,
    pub decals: Vec<SDecalSet>,
    pub unk18: Vec<()>,
    pub vb0: TagHash,
    pub vb1: TagHash,
    pub unk30: u32,
    pub unk34: u32,
    pub decal_bounds: Tag<SOcclusionBounds>,
    pub unk3c: u32,
    pub bounds: AxisAlignedBBox,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x8080822C)]
pub struct SDecalSet {
    pub technique: TagHash,
    pub start: u16,
    pub count: u16,
}
