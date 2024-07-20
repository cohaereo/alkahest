use destiny_pkg::TagHash;
use tiger_parse::tiger_tag;

use crate::dxgi::DxgiFormat;

#[derive(Debug)]
#[tiger_tag(etype = 32, size = 0x28)]
pub struct STextureHeader {
    pub data_size: u32,
    pub format: DxgiFormat,
    pub _unk8: u32,

    pub cafe: u16, // 0xc

    pub width: u16,      // 0xe
    pub height: u16,     // 0x10
    pub depth: u16,      // 0x12
    pub array_size: u16, // 0x14

    pub unk16: u8,
    pub mip_count: u8,
    pub unk18: [u8; 12],

    /// Optional
    pub large_buffer: TagHash,
}
