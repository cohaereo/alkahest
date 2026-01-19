use glam::Vec4;
use tiger_parse::{tiger_type, tiger_variant_enum, FnvHash, VariantPointer};
use tiger_pkg::TagHash;

#[derive(Debug, Clone)]
#[tiger_type(id = 0x80808179, size = 0x220)]
pub struct SUnk80808179 {
    #[tiger(offset = 0x1C8)]
    pub unk1c8: Vec<SUnk808091f1>,
    pub unk1d8: Vec<SUnk808091f1>,
    // pub unk1e8: Vec<SUnk808084df>,
    #[tiger(offset = 0x1F8)]
    pub unk1f8: Vec<SUnk8080816f>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x8080816f, size = 0x38)]
pub struct SUnk8080816f {
    pub unk0: TagHash,
    pub unk4: u32,
    pub unk8: i64,
    pub unk10: i64,
    pub unk18: TagHash,
    #[tiger(offset = 0x30)]
    pub unk30: FnvHash,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808091f1, size = 0x18)]
pub struct SUnk808091f1 {
    #[tiger(offset = 0x10)]
    pub unk18: VariantPointer<SUnk808091f1Variant>,
}

tiger_variant_enum! {
    #[derive(Debug, Clone)]
    [Unknown(true)]
    enum SUnk808091f1Variant {
        SSequenceGlobalChannel,
        SUnk808091df,
        SUnk808091db,
        SUnk808091dd
    }
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808091d1, size = 0x60)]
pub struct SSequenceGlobalChannel {
    pub base: SSequenceNodeBase,
    pub unk20: u32,
    pub unk24: u32,
    pub other_index: u32,
    pub unk2c: FnvHash,

    pub bytecode: Vec<u8>,
    pub bytecode_constants: Vec<Vec4>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808091dd, size = 0x60)]
pub struct SUnk808091dd {
    pub base: SSequenceNodeBase,
    pub children: Vec<(u16, u16)>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808091df, size = 0x60)]
pub struct SUnk808091df {
    pub base: SSequenceNodeBase,
    pub children: Vec<(u16, u16)>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808091db, size = 0x60)]
pub struct SUnk808091db {
    pub base: SSequenceNodeBase,
    pub children: Vec<(u16, u16)>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x00000000)]
pub struct SSequenceNodeBase {
    pub name: FnvHash,
    pub unk4: u16,
    pub parent_index: u16,
    pub unk8: u32,

    pub unkc: f32,
    pub start_time: f32,
    pub unk14: f32,
    pub duration: f32,
    pub unk1c: u32,
}
