use destiny_pkg::TagHash;
use tiger_parse::tiger_tag;

use crate::dxgi::DxgiFormat;

#[derive(Debug)]
#[tiger_tag(etype = 32, size = 0x40)]
pub struct STextureHeader {
    pub data_size: u32,
    pub format: DxgiFormat,
    pub _unk8: u32,

    #[tag(offset = 0x20)]
    pub cafe: u16,

    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub array_size: u16,

    pub unk2a: u16,
    pub unk2c: u8,
    pub mip_count: u8,
    pub unk2e: [u8; 10],
    pub unk38: u32,

    /// Optional
    pub large_buffer: TagHash,
}

#[derive(Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct TexturePlate {
    pub file_size: u64,
    pub _unk: u64,
    pub transforms: Vec<TexturePlateTransform>,
}

#[derive(Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct TexturePlateTransform {
    pub texture: TagHash,
    pub translation: glam::IVec2,
    pub dimensions: glam::IVec2,
}

#[derive(Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct TexturePlateSet {
    pub file_size: u64,
    pub _unk: [u32; 7],
    pub diffuse: TagHash,
    pub normal: TagHash,
    pub gstack: TagHash,
}
