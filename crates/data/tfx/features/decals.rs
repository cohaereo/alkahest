use tiger_parse::{tiger_type, Padding};
use tiger_pkg::TagHash;

use crate::{
    tag::Tag,
    tfx::{
        common::{AxisAlignedBBox, SOcclusionBounds},
        RenderStage,
    },
};

#[derive(Clone, Debug)]
#[tiger_type(id = 0x8080695B)]
pub struct SDecalCollection {
    pub file_size: u64,
    pub decals: Vec<SDecalSet>,
    pub unk18: Vec<()>,
    pub vb0: TagHash,
    pub vb1: TagHash,
    pub unk30: u16,
    pub unk32: u16,

    pub unk34: u16,
    pub render_stage: RenderStage,
    _pad37: Padding<1>,

    pub decal_bounds: Tag<SOcclusionBounds>,
    _pad3c: Padding<4>,
    pub bounds: AxisAlignedBBox,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80806963)]
pub struct SDecalSet {
    pub technique: TagHash,
    pub start: u16,
    pub count: u16,
}
