use glam::Vec4;
use tiger_parse::{tiger_type, tiger_variant_enum, FnvHash, VariantPointer};

#[derive(Debug, Clone)]
#[tiger_type(id = 0x80808179, size = 0x200)]
pub struct SUnk80808179 {
    #[tiger(offset = 0x1C8)]
    pub unk1c8: Vec<SUnk808091f1>,
    pub unk1d8: Vec<SUnk808091f1>,
    // pub unk178: Vec<SUnk808084df>,
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
        SSequenceGlobalChannel
    }
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808091d1, size = 0x60)]
pub struct SSequenceGlobalChannel {
    pub channel_hash: FnvHash,
    pub unk4: [u32; 3],
    pub unk10: [u32; 4],
    pub unk20: [u32; 3],
    pub unk2c: FnvHash,

    pub bytecode: Vec<u8>,
    pub bytecode_constants: Vec<Vec4>,
}
