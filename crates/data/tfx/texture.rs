use d3d11::dxgi;
use tiger_parse::{tiger_type, TigerReadable};
use tiger_pkg::TagHash;

#[derive(Debug)]
#[tiger_type(etype = 32, size = 0x40)]
pub struct STextureHeader {
    pub data_size: u32,
    pub format: DxgiFormat,
    pub _unk8: u32,

    #[tiger(offset = 0x20)]
    pub cafe: u16,

    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub array_size: u16,
    pub tile_count: u16,

    pub unk2c: u8,
    pub mip_count: u8,
    pub unk2e: [u8; 10],
    pub unk38: u32,

    /// Optional
    pub large_buffer: TagHash,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DxgiFormat(d3d11::dxgi::Format);

impl TigerReadable for DxgiFormat {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let v = u32::read_ds_endian(reader, endian)?;
        Self::try_from(v).map_err(|_| tiger_parse::Error::EnumVariantOutOfRange(v as usize))
    }

    const SIZE: usize = 4;
}

impl From<DxgiFormat> for u32 {
    fn from(val: DxgiFormat) -> Self {
        val.0 as u32
    }
}

impl From<DxgiFormat> for d3d11::dxgi::Format {
    fn from(val: DxgiFormat) -> Self {
        val.0
    }
}

impl From<d3d11::dxgi::Format> for DxgiFormat {
    fn from(val: d3d11::dxgi::Format) -> Self {
        DxgiFormat(val)
    }
}

impl TryFrom<u32> for DxgiFormat {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match dxgi::Format::try_from(value) {
            Ok(o) => Ok(Self(o)),
            Err(_) => Err(anyhow::anyhow!("DXGI format out of range")),
        }
    }
}

#[allow(unused)]
impl DxgiFormat {
    pub fn bpp(&self) -> u32 {
        match self.0 {
            dxgi::Format::R32g32b32a32Typeless
            | dxgi::Format::R32g32b32a32Float
            | dxgi::Format::R32g32b32a32Uint
            | dxgi::Format::R32g32b32a32Sint => 128,
            dxgi::Format::R32g32b32Typeless
            | dxgi::Format::R32g32b32Float
            | dxgi::Format::R32g32b32Uint
            | dxgi::Format::R32g32b32Sint => 96,
            dxgi::Format::R16g16b16a16Typeless
            | dxgi::Format::R16g16b16a16Float
            | dxgi::Format::R16g16b16a16Unorm
            | dxgi::Format::R16g16b16a16Uint
            | dxgi::Format::R16g16b16a16Snorm
            | dxgi::Format::R16g16b16a16Sint
            | dxgi::Format::R32g32Typeless
            | dxgi::Format::R32g32Float
            | dxgi::Format::R32g32Uint
            | dxgi::Format::R32g32Sint
            | dxgi::Format::R32g8x24Typeless
            | dxgi::Format::D32FloatS8x24Uint
            | dxgi::Format::R32FloatX8x24Typeless
            | dxgi::Format::X32TypelessG8x24Uint
            | dxgi::Format::Y416
            | dxgi::Format::Y210
            | dxgi::Format::Y216 => 64,
            dxgi::Format::R10g10b10a2Typeless
            | dxgi::Format::R10g10b10a2Unorm
            | dxgi::Format::R10g10b10a2Uint
            | dxgi::Format::R11g11b10Float
            | dxgi::Format::R8g8b8a8Typeless
            | dxgi::Format::R8g8b8a8Unorm
            | dxgi::Format::R8g8b8a8UnormSrgb
            | dxgi::Format::R8g8b8a8Uint
            | dxgi::Format::R8g8b8a8Snorm
            | dxgi::Format::R8g8b8a8Sint
            | dxgi::Format::R16g16Typeless
            | dxgi::Format::R16g16Float
            | dxgi::Format::R16g16Unorm
            | dxgi::Format::R16g16Uint
            | dxgi::Format::R16g16Snorm
            | dxgi::Format::R16g16Sint
            | dxgi::Format::R32Typeless
            | dxgi::Format::D32Float
            | dxgi::Format::R32Float
            | dxgi::Format::R32Uint
            | dxgi::Format::R32Sint
            | dxgi::Format::R24g8Typeless
            | dxgi::Format::D24UnormS8Uint
            | dxgi::Format::R24UnormX8Typeless
            | dxgi::Format::X24TypelessG8Uint
            | dxgi::Format::R9g9b9e5Sharedexp
            | dxgi::Format::R8g8B8g8Unorm
            | dxgi::Format::G8r8G8b8Unorm
            | dxgi::Format::B8g8r8a8Unorm
            | dxgi::Format::B8g8r8x8Unorm
            | dxgi::Format::R10g10b10XrBiasA2Unorm
            | dxgi::Format::B8g8r8a8Typeless
            | dxgi::Format::B8g8r8a8UnormSrgb
            | dxgi::Format::B8g8r8x8Typeless
            | dxgi::Format::B8g8r8x8UnormSrgb
            | dxgi::Format::Ayuv
            | dxgi::Format::Y410
            | dxgi::Format::Yuy2 => 32,
            dxgi::Format::P010 | dxgi::Format::P016 => 24,
            dxgi::Format::R8g8Typeless
            | dxgi::Format::R8g8Unorm
            | dxgi::Format::R8g8Uint
            | dxgi::Format::R8g8Snorm
            | dxgi::Format::R8g8Sint
            | dxgi::Format::R16Typeless
            | dxgi::Format::R16Float
            | dxgi::Format::D16Unorm
            | dxgi::Format::R16Unorm
            | dxgi::Format::R16Uint
            | dxgi::Format::R16Snorm
            | dxgi::Format::R16Sint
            | dxgi::Format::B5g6r5Unorm
            | dxgi::Format::B5g5r5a1Unorm
            | dxgi::Format::A8p8
            | dxgi::Format::B4g4r4a4Unorm => 16,
            dxgi::Format::Nv12 | dxgi::Format::Opaque420 | dxgi::Format::Nv11 => 12,
            dxgi::Format::R8Typeless
            | dxgi::Format::R8Unorm
            | dxgi::Format::R8Uint
            | dxgi::Format::R8Snorm
            | dxgi::Format::R8Sint
            | dxgi::Format::A8Unorm
            | dxgi::Format::Ai44
            | dxgi::Format::Ia44
            | dxgi::Format::P8 => 8,
            dxgi::Format::R1Unorm => 1,
            dxgi::Format::Bc1Typeless
            | dxgi::Format::Bc1Unorm
            | dxgi::Format::Bc1UnormSrgb
            | dxgi::Format::Bc4Typeless
            | dxgi::Format::Bc4Unorm
            | dxgi::Format::Bc4Snorm => 4,
            dxgi::Format::Bc2Typeless
            | dxgi::Format::Bc2Unorm
            | dxgi::Format::Bc2UnormSrgb
            | dxgi::Format::Bc3Typeless
            | dxgi::Format::Bc3Unorm
            | dxgi::Format::Bc3UnormSrgb
            | dxgi::Format::Bc5Typeless
            | dxgi::Format::Bc5Unorm
            | dxgi::Format::Bc5Snorm
            | dxgi::Format::Bc6hTypeless
            | dxgi::Format::Bc6hUf16
            | dxgi::Format::Bc6hSf16
            | dxgi::Format::Bc7Typeless
            | dxgi::Format::Bc7Unorm
            | dxgi::Format::Bc7UnormSrgb => 8,
            u => panic!("{u:?}"),
        }
    }

    pub fn is_srgb(&self) -> bool {
        matches!(
            self.0,
            dxgi::Format::R8g8b8a8UnormSrgb
                | dxgi::Format::Bc1UnormSrgb
                | dxgi::Format::Bc2UnormSrgb
                | dxgi::Format::Bc3UnormSrgb
                | dxgi::Format::B8g8r8a8UnormSrgb
                | dxgi::Format::B8g8r8x8UnormSrgb
                | dxgi::Format::Bc7UnormSrgb
        )
    }

    pub fn is_compressed(&self) -> bool {
        matches!(
            self.0,
            dxgi::Format::Bc1Typeless
                | dxgi::Format::Bc1Unorm
                | dxgi::Format::Bc1UnormSrgb
                | dxgi::Format::Bc4Typeless
                | dxgi::Format::Bc4Unorm
                | dxgi::Format::Bc4Snorm
                | dxgi::Format::Bc2Typeless
                | dxgi::Format::Bc2Unorm
                | dxgi::Format::Bc2UnormSrgb
                | dxgi::Format::Bc3Typeless
                | dxgi::Format::Bc3Unorm
                | dxgi::Format::Bc3UnormSrgb
                | dxgi::Format::Bc5Typeless
                | dxgi::Format::Bc5Unorm
                | dxgi::Format::Bc5Snorm
                | dxgi::Format::Bc6hTypeless
                | dxgi::Format::Bc6hUf16
                | dxgi::Format::Bc6hSf16
                | dxgi::Format::Bc7Typeless
                | dxgi::Format::Bc7Unorm
                | dxgi::Format::Bc7UnormSrgb
        )
    }

    pub fn calculate_pitch(&self, width: u32, height: u32) -> (u32, u32) {
        match self.0 {
            dxgi::Format::Bc1Typeless
            | dxgi::Format::Bc1Unorm
            | dxgi::Format::Bc1UnormSrgb
            | dxgi::Format::Bc4Typeless
            | dxgi::Format::Bc4Unorm
            | dxgi::Format::Bc4Snorm => {
                let nbw = ((width as i64 + 3) / 4).clamp(1, i64::MAX) as u32;
                let nbh = ((height as i64 + 3) / 4).clamp(1, i64::MAX) as u32;

                let pitch = nbw * 8;
                (pitch, pitch * nbh)
            }
            dxgi::Format::Bc2Typeless
            | dxgi::Format::Bc2Unorm
            | dxgi::Format::Bc2UnormSrgb
            | dxgi::Format::Bc3Typeless
            | dxgi::Format::Bc3Unorm
            | dxgi::Format::Bc3UnormSrgb
            | dxgi::Format::Bc5Typeless
            | dxgi::Format::Bc5Unorm
            | dxgi::Format::Bc5Snorm
            | dxgi::Format::Bc6hTypeless
            | dxgi::Format::Bc6hUf16
            | dxgi::Format::Bc6hSf16
            | dxgi::Format::Bc7Typeless
            | dxgi::Format::Bc7Unorm
            | dxgi::Format::Bc7UnormSrgb => {
                let nbw = ((width as i64 + 3) / 4).clamp(1, i64::MAX) as u32;
                let nbh = ((height as i64 + 3) / 4).clamp(1, i64::MAX) as u32;

                let pitch = nbw * 16;
                (pitch, pitch * nbh)
            }
            _ => {
                let pitch = (width * self.bpp()).div_ceil(8);
                (pitch, height * pitch)
            }
        }
    }
}
