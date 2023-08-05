use crate::structure::TablePointer;
use crate::types::Vector4;
use binrw::BinRead;
use destiny_pkg::TagHash;

#[derive(BinRead, Debug)]
pub struct Unk808071e8 {
    pub file_size: u64,
    pub unk8: [u32; 16],

    pub vertex_shader: TagHash,
    pub unk24c: u32,
    pub vs_textures: TablePointer<Unk80807211>,
    pub unk60: u32,
    pub unk64: u32,
    pub unk68: TablePointer<u8>,
    pub unk78: TablePointer<Vector4>,
    pub vs_samplers: TablePointer<Unk808073f3>,
    pub unk98: TablePointer<Vector4>,
    pub unka8: [u32; 9],

    pub unkcc: TagHash,
    pub unkd0: [u32; 126],

    pub pixel_shader: TagHash,
    pub unk2cc: u32,
    pub ps_textures: TablePointer<Unk80807211>,
    pub unk2e0: u32,
    pub unk2e4: u32,
    pub unk2e8: TablePointer<u8>,
    pub unk2f8: TablePointer<Vector4>,
    pub ps_samplers: TablePointer<Unk808073f3>,
    pub unk318: TablePointer<Vector4>,
    pub unk328: [u32; 9],

    /// Pointer to a float4 buffer, usually passed into cbuffer0
    pub unk34c: TagHash,
}

#[derive(BinRead, Debug)]
pub struct Unk80807211 {
    /// Material slot to assign to
    pub index: u32,
    pub texture: TagHash,
}

#[derive(BinRead, Debug)]
pub struct Unk808073f3 {
    pub sampler: TagHash,
    pub unk4: u32,
    pub unk8: u32,
    pub unkc: u32,
}
