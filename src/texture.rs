use crate::dxgi::DxgiFormat;
use crate::structure::{CafeMarker, TablePointer};
use crate::types::IVector2;
use binrw::BinRead;
use destiny_pkg::TagHash;
use std::io::SeekFrom;
use windows::Win32::Graphics::Direct3D11::{ID3D11Texture2D, ID3D11Texture3D};

#[derive(BinRead, Debug)]
pub struct TextureHeader {
    pub data_size: u32,
    pub format: DxgiFormat,
    pub _unk8: u32,

    pub cafe: CafeMarker,

    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub array_size: u16,

    #[br(seek_before = SeekFrom::Start(0x24))]
    #[br(map(|v: u32| (v != u32::MAX).then_some(TagHash(v))))]
    pub large_buffer: Option<TagHash>,
}

/// Ref: 0x80809ebb
#[derive(BinRead, Debug)]
pub struct TexturePlate {
    pub file_size: u64,
    pub _unk: u64,
    pub transforms: TablePointer<TexturePlateTransform>,
}

#[derive(BinRead, Debug)]
pub struct TexturePlateTransform {
    pub texture: TagHash,
    pub translation: IVector2,
    pub dimensions: IVector2,
}

/// Ref: 0x808072d2
#[derive(BinRead, Debug)]
pub struct TexturePlateSet {
    pub file_size: u64,
    pub _unk: [u32; 7],
    pub diffuse: TagHash,
    pub normal: TagHash,
    pub gstack: TagHash,
}

pub enum TextureHandle {
    Texture2D(ID3D11Texture2D),
    Texture3D(ID3D11Texture3D),
}
