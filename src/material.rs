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
    pub unk60: [u32; 154],

    pub pixel_shader: TagHash,
    pub unk2cc: u32,
    pub ps_textures: TablePointer<Unk80807211>,
    pub unk2e0: u32,
    pub unk2e4: u32,
    pub unk2e8: TablePointer<u8>,
    pub unk2f8: u64,
    pub unk300: u64,
    pub unk308: TablePointer<Unk808073f3>,
    pub unk318: TablePointer<Vector4>,
}

#[derive(BinRead, Debug)]
pub struct Unk80807211 {
    pub index: u32,
    pub texture: TagHash,
}

#[derive(BinRead, Debug)]
pub struct Unk808073f3 {
    pub unk0: TagHash,
    pub unk4: u32,
    pub unk8: u32,
    pub unkc: u32,
}
