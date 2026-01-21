use glam::Vec4;
use tiger_parse::{tiger_type, tiger_variant_enum, VariantPointer};
use tiger_pkg::TagHash;

use crate::tag::{OptionalTag, Tag, WideHash};

#[derive(Debug)]
#[tiger_type(id = 0x80806BC1)]
pub struct SAtmosphereDataComponent {
    pub unk0: [u32; 4 * 8],
    pub unk80_tex: WideHash,
    pub unk90_tex: WideHash,
    pub unka0_tex: WideHash,
    pub unkb0_tex: WideHash,
    pub unkc0_tex: TagHash,
    pub unkc4: TagHash,

    pub unkc8: [f32; 4 * 4],
}

#[derive(Debug)]
#[tiger_type(id = 0x80806A74)]
pub struct SUnk80806a74 {
    pub unk0: Vec4,
    pub unk10: Tag<SUnk80808ac8>,
    pub unk14: OptionalTag<SUnk80808ac8>,
    pub unk18: Tag<SUnk80808ac8>,
    pub unk1c: OptionalTag<SUnk80808ac8>,
}

#[derive(Debug)]
#[tiger_type(id = 0x80808AC8)]
pub struct SUnk80808ac8 {
    pub file_size: u64,
    pub unk8: u32,
    pub unkc: f32,
    pub unk10: VariantPointer<SUnk80808ac8Variant>,
}

tiger_variant_enum! {
    #[derive(Debug)]
    [offset = 0x10]
    enum SUnk80808ac8Variant {
        SSunAngles
    }
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80808B49)]
pub struct SSunAngles {
    pub angles: Vec<Vec4>,
}
