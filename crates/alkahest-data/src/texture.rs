use destiny_pkg::TagHash;
use tiger_parse::tiger_tag;

use crate::dxgi::DxgiFormat;

#[derive(Debug)]
#[tiger_tag(etype = 32, size = 0x28)]
pub struct STextureHeader {
    pub data_size: u32,
    pub format: DxgiFormat,
    pub _unk8: u32,

    pub cafe: u16,

    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub array_size: u16,

    pub unk16: u16,
    pub unk18: u8,
    pub mip_count: u8,
    pub unk2e: [u8; 10],

    /// Optional
    pub large_buffer: TagHash,
}

