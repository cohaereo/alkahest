use glam::Vec4;
use tiger_parse::{tiger_type, tiger_variant_enum, FnvHash, VariantPointer};
use tiger_pkg::TagHash;

use crate::tag::WideHash;

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
        SUnk808091e3,
        SUnk808091df,
        SUnk808091db,
        SUnk808091dd,
        SSequenceLight,
        SSequenceLensFlare,
        // SSequenceEmbeddedParticleSystem,
        SSequenceAudioEvent
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
#[tiger_type(id = 0x808091e3, size = 0x60)]
pub struct SUnk808091e3 {
    pub base: SSequenceNodeBase,
    pub children: Vec<(u16, u16)>,
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
#[tiger_type(id = 0x80806a52, size = 0x130)]
pub struct SSequenceLight {
    pub base: SSequenceNodeBase,
    pub unk20: u32,
    pub light: TagHash,
    pub unk28: u32,
    pub unk2c: u32,
    pub unk30: Vec4,
    pub unk40: Vec4,
    pub unk50: u64,

    pub unk58: SUnknownEventExpressions,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x80806a48, size = 0x130)]
pub struct SSequenceLensFlare {
    pub base: SSequenceNodeBase,
    pub unk20: u32,
    pub flare: TagHash,

    #[tiger(offset = 0x40)]
    pub unk40: SUnknownEventExpressions,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x80806640, size = 0x60)]
pub struct SSequenceAudioEvent {
    pub base: SSequenceNodeBase,
    #[tiger(offset = 0x50)]
    pub wwise_event: WideHash,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808067b9, size = 0x110)]
pub struct SSequenceEmbeddedParticleSystem {
    pub base: SSequenceNodeBase,
    pub unk20: u64,
    pub unk28: Vec<SUnk808067bb>,

    pub unk38: SUnknownEventExpressions,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808067bb, size = 0x20)]
pub struct SUnk808067bb {
    pub unk0: Vec<u8>,
    pub particle_system: TagHash,
    pub unk14: TagHash,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x00000000, size = 0xd8)]
pub struct SUnknownEventExpressions {
    pub unk00: Vec<u8>,
    pub unk10: Vec<Vec4>,

    #[tiger(offset = 0x48)]
    pub unk48: Vec<u8>,
    pub unk58: Vec<Vec4>,

    #[tiger(offset = 0x90)]
    pub unk88: Vec<u8>,
    pub unk98: Vec<Vec4>,
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
