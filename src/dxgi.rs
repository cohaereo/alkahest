use binrw::BinRead;
use std::mem::transmute;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;
// use vulkano::format::Format as VkFormat;

#[allow(non_camel_case_types, dead_code, clippy::upper_case_acronyms)]
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, BinRead)]
#[br(repr(u32))]
pub enum DxgiFormat {
    Unknown = 0,
    R32G32B32A32_TYPELESS = 1,
    R32G32B32A32_FLOAT = 2,
    R32G32B32A32_UINT = 3,
    R32G32B32A32_SINT = 4,
    R32G32B32_TYPELESS = 5,
    R32G32B32_FLOAT = 6,
    R32G32B32_UINT = 7,
    R32G32B32_SINT = 8,
    R16G16B16A16_TYPELESS = 9,
    R16G16B16A16_FLOAT = 10,
    R16G16B16A16_UNORM = 11,
    R16G16B16A16_UINT = 12,
    R16G16B16A16_SNORM = 13,
    R16G16B16A16_SINT = 14,
    R32G32_TYPELESS = 15,
    R32G32_FLOAT = 16,
    R32G32_UINT = 17,
    R32G32_SINT = 18,
    R32G8X24_TYPELESS = 19,
    D32_FLOAT_S8X24_UINT = 20,
    R32_FLOAT_X8X24_TYPELESS = 21,
    X32_TYPELESS_G8X24_UINT = 22,
    R10G10B10A2_TYPELESS = 23,
    R10G10B10A2_UNORM = 24,
    R10G10B10A2_UINT = 25,
    R11G11B10_FLOAT = 26,
    R8G8B8A8_TYPELESS = 27,
    R8G8B8A8_UNORM = 28,
    R8G8B8A8_UNORM_SRGB = 29,
    R8G8B8A8_UINT = 30,
    R8G8B8A8_SNORM = 31,
    R8G8B8A8_SINT = 32,
    R16G16_TYPELESS = 33,
    R16G16_FLOAT = 34,
    R16G16_UNORM = 35,
    R16G16_UINT = 36,
    R16G16_SNORM = 37,
    R16G16_SINT = 38,
    R32_TYPELESS = 39,
    D32_FLOAT = 40,
    R32_FLOAT = 41,
    R32_UINT = 42,
    R32_SINT = 43,
    R24G8_TYPELESS = 44,
    D24_UNORM_S8_UINT = 45,
    R24_UNORM_X8_TYPELESS = 46,
    X24_TYPELESS_G8_UINT = 47,
    R8G8_TYPELESS = 48,
    R8G8_UNORM = 49,
    R8G8_UINT = 50,
    R8G8_SNORM = 51,
    R8G8_SINT = 52,
    R16_TYPELESS = 53,
    R16_FLOAT = 54,
    D16_UNORM = 55,
    R16_UNORM = 56,
    R16_UINT = 57,
    R16_SNORM = 58,
    R16_SINT = 59,
    R8_TYPELESS = 60,
    R8_UNORM = 61,
    R8_UINT = 62,
    R8_SNORM = 63,
    R8_SINT = 64,
    A8_UNORM = 65,
    R1_UNORM = 66,
    R9G9B9E5_SHAREDEXP = 67,
    R8G8_B8G8_UNORM = 68,
    G8R8_G8B8_UNORM = 69,
    BC1_TYPELESS = 70,
    BC1_UNORM = 71,
    BC1_UNORM_SRGB = 72,
    BC2_TYPELESS = 73,
    BC2_UNORM = 74,
    BC2_UNORM_SRGB = 75,
    BC3_TYPELESS = 76,
    BC3_UNORM = 77,
    BC3_UNORM_SRGB = 78,
    BC4_TYPELESS = 79,
    BC4_UNORM = 80,
    BC4_SNORM = 81,
    BC5_TYPELESS = 82,
    BC5_UNORM = 83,
    BC5_SNORM = 84,
    B5G6R5_UNORM = 85,
    B5G5R5A1_UNORM = 86,
    B8G8R8A8_UNORM = 87,
    B8G8R8X8_UNORM = 88,
    R10G10B10_XR_BIAS_A2_UNORM = 89,
    B8G8R8A8_TYPELESS = 90,
    B8G8R8A8_UNORM_SRGB = 91,
    B8G8R8X8_TYPELESS = 92,
    B8G8R8X8_UNORM_SRGB = 93,
    BC6H_TYPELESS = 94,
    BC6H_UF16 = 95,
    BC6H_SF16 = 96,
    BC7_TYPELESS = 97,
    BC7_UNORM = 98,
    BC7_UNORM_SRGB = 99,
    AYUV = 100,
    Y410 = 101,
    Y416 = 102,
    NV12 = 103,
    P010 = 104,
    P016 = 105,
    OPAQUE420 = 106,
    YUY2 = 107,
    Y210 = 108,
    Y216 = 109,
    NV11 = 110,
    AI44 = 111,
    IA44 = 112,
    P8 = 113,
    A8P8 = 114,
    B4G4R4A4_UNORM = 115,
    P208 = 130,
    V208 = 131,
    V408 = 132,
    SAMPLER_FEEDBACK_MIN_MIP_OPAQUE,
    SAMPLER_FEEDBACK_MIP_REGION_USED_OPAQUE,
    FORCE_UINT = 0xffffffff,
}

impl From<DxgiFormat> for DXGI_FORMAT {
    fn from(val: DxgiFormat) -> Self {
        DXGI_FORMAT(val.into())
    }
}

impl From<DxgiFormat> for u32 {
    fn from(val: DxgiFormat) -> Self {
        unsafe { transmute(val) }
    }
}

impl TryFrom<u32> for DxgiFormat {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0..=115 | 130..=132 => unsafe { transmute(value) },
            e => return Err(anyhow::anyhow!("DXGI format is out of range ({e})")),
        })
    }
}

#[allow(unused)]
impl DxgiFormat {
    // https://learn.microsoft.com/en-us/windows/win32/api/dxgiformat/ne-dxgiformat-dxgi_format#syntax
    // https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkFormat.html#_c_specification
    // pub fn to_vulkan(&self) -> anyhow::Result<VkFormat> {
    //     Ok(match self {
    //         DxgiFormat::R32G32B32A32_TYPELESS => VkFormat::R32G32B32A32_SFLOAT,
    //         DxgiFormat::R32G32B32A32_FLOAT => VkFormat::R32G32B32A32_SFLOAT,
    //         DxgiFormat::R32G32B32A32_UINT => VkFormat::R32G32B32A32_UINT,
    //         DxgiFormat::R32G32B32A32_SINT => VkFormat::R32G32B32A32_SINT,
    //         DxgiFormat::R32G32B32_TYPELESS => VkFormat::R32G32B32_SFLOAT,
    //         DxgiFormat::R32G32B32_FLOAT => VkFormat::R32G32B32_SFLOAT,
    //         DxgiFormat::R32G32B32_UINT => VkFormat::R32G32B32_UINT,
    //         DxgiFormat::R32G32B32_SINT => VkFormat::R32G32B32_SINT,
    //         DxgiFormat::R16G16B16A16_TYPELESS => VkFormat::R16G16B16A16_SFLOAT,
    //         DxgiFormat::R16G16B16A16_FLOAT => VkFormat::R16G16B16A16_SFLOAT,
    //         DxgiFormat::R16G16B16A16_UNORM => VkFormat::R16G16B16A16_UNORM,
    //         DxgiFormat::R16G16B16A16_UINT => VkFormat::R16G16B16A16_UINT,
    //         DxgiFormat::R16G16B16A16_SNORM => VkFormat::R16G16B16A16_SNORM,
    //         DxgiFormat::R16G16B16A16_SINT => VkFormat::R16G16B16A16_SINT,
    //         DxgiFormat::R32G32_TYPELESS => VkFormat::R32G32_SFLOAT,
    //         DxgiFormat::R32G32_FLOAT => VkFormat::R32G32_SFLOAT,
    //         DxgiFormat::R32G32_UINT => VkFormat::R32G32_UINT,
    //         DxgiFormat::R32G32_SINT => VkFormat::R32G32_SINT,
    //         DxgiFormat::R10G10B10A2_TYPELESS => VkFormat::A2B10G10R10_UNORM_PACK32,
    //         DxgiFormat::R10G10B10A2_UNORM => VkFormat::A2B10G10R10_UNORM_PACK32,
    //         DxgiFormat::R10G10B10A2_UINT => VkFormat::A2B10G10R10_UINT_PACK32,
    //         DxgiFormat::R11G11B10_FLOAT => VkFormat::B10G11R11_UFLOAT_PACK32,
    //         DxgiFormat::R8G8_TYPELESS => VkFormat::R8G8_UNORM,
    //         DxgiFormat::R8G8_UNORM => VkFormat::R8G8_UNORM,
    //         DxgiFormat::R8G8_UINT => VkFormat::R8G8_UINT,
    //         DxgiFormat::R8G8_SNORM => VkFormat::R8G8_SNORM,
    //         DxgiFormat::R8G8_SINT => VkFormat::R8G8_SINT,
    //         DxgiFormat::R8G8B8A8_TYPELESS => VkFormat::R8G8B8A8_UNORM,
    //         DxgiFormat::R8G8B8A8_UNORM => VkFormat::R8G8B8A8_UNORM,
    //         DxgiFormat::R8G8B8A8_UNORM_SRGB => VkFormat::R8G8B8A8_SRGB,
    //         DxgiFormat::R8G8B8A8_UINT => VkFormat::R8G8B8A8_UINT,
    //         DxgiFormat::R8G8B8A8_SNORM => VkFormat::R8G8B8A8_SNORM,
    //         DxgiFormat::R8G8B8A8_SINT => VkFormat::R8G8B8A8_SINT,
    //         DxgiFormat::R16G16_TYPELESS => VkFormat::R16G16_SFLOAT,
    //         DxgiFormat::R16G16_FLOAT => VkFormat::R16G16_SFLOAT,
    //         DxgiFormat::R16G16_UNORM => VkFormat::R16G16_UNORM,
    //         DxgiFormat::R16G16_UINT => VkFormat::R16G16_UINT,
    //         DxgiFormat::R16G16_SNORM => VkFormat::R16G16_SNORM,
    //         DxgiFormat::R16G16_SINT => VkFormat::R16G16_SINT,
    //         DxgiFormat::R32_TYPELESS => VkFormat::R32_UINT,
    //         DxgiFormat::D32_FLOAT => VkFormat::D32_SFLOAT,
    //         DxgiFormat::R32_FLOAT => VkFormat::R32_SFLOAT,
    //         DxgiFormat::R32_UINT => VkFormat::R32_UINT,
    //         DxgiFormat::R32_SINT => VkFormat::R32_SINT,
    //         DxgiFormat::R16_TYPELESS => VkFormat::R16_UINT,
    //         DxgiFormat::R16_FLOAT => VkFormat::R16_SFLOAT,
    //         DxgiFormat::D16_UNORM => VkFormat::D16_UNORM,
    //         DxgiFormat::R16_UNORM => VkFormat::R16_UNORM,
    //         DxgiFormat::R16_UINT => VkFormat::R16_UINT,
    //         DxgiFormat::R16_SNORM => VkFormat::R16_SNORM,
    //         DxgiFormat::R16_SINT => VkFormat::R16_SINT,
    //         DxgiFormat::R8_TYPELESS => VkFormat::R8_UNORM,
    //         DxgiFormat::R8_UNORM => VkFormat::R8_UNORM,
    //         DxgiFormat::R8_UINT => VkFormat::R8_UINT,
    //         DxgiFormat::R8_SNORM => VkFormat::R8_SNORM,
    //         DxgiFormat::R8_SINT => VkFormat::R8_SINT,
    //         DxgiFormat::A8_UNORM => VkFormat::R8_UNORM,
    //         DxgiFormat::B8G8R8A8_UNORM => VkFormat::B8G8R8A8_UNORM,
    //         DxgiFormat::B8G8R8X8_UNORM => VkFormat::B8G8R8A8_UNORM,
    //         DxgiFormat::B8G8R8A8_TYPELESS => VkFormat::B8G8R8A8_UNORM,
    //         DxgiFormat::B8G8R8A8_UNORM_SRGB => VkFormat::B8G8R8A8_SRGB,
    //         DxgiFormat::B8G8R8X8_TYPELESS => VkFormat::B8G8R8A8_UNORM,
    //         DxgiFormat::B8G8R8X8_UNORM_SRGB => VkFormat::B8G8R8A8_SRGB,
    //         DxgiFormat::R9G9B9E5_SHAREDEXP => VkFormat::E5B9G9R9_UFLOAT_PACK32,
    //         DxgiFormat::B5G6R5_UNORM => VkFormat::R5G6B5_UNORM_PACK16,
    //         DxgiFormat::B5G5R5A1_UNORM => VkFormat::A1R5G5B5_UNORM_PACK16,
    //         DxgiFormat::BC1_TYPELESS => VkFormat::BC1_RGBA_UNORM_BLOCK,
    //         DxgiFormat::BC1_UNORM => VkFormat::BC1_RGBA_UNORM_BLOCK,
    //         DxgiFormat::BC1_UNORM_SRGB => VkFormat::BC1_RGBA_SRGB_BLOCK,
    //         DxgiFormat::BC2_TYPELESS => VkFormat::BC2_UNORM_BLOCK,
    //         DxgiFormat::BC2_UNORM => VkFormat::BC2_UNORM_BLOCK,
    //         DxgiFormat::BC2_UNORM_SRGB => VkFormat::BC2_SRGB_BLOCK,
    //         DxgiFormat::BC3_TYPELESS => VkFormat::BC3_UNORM_BLOCK,
    //         DxgiFormat::BC3_UNORM => VkFormat::BC3_UNORM_BLOCK,
    //         DxgiFormat::BC3_UNORM_SRGB => VkFormat::BC3_SRGB_BLOCK,
    //         DxgiFormat::BC4_TYPELESS => VkFormat::BC4_UNORM_BLOCK,
    //         DxgiFormat::BC4_UNORM => VkFormat::BC4_UNORM_BLOCK,
    //         DxgiFormat::BC4_SNORM => VkFormat::BC4_SNORM_BLOCK,
    //         DxgiFormat::BC5_TYPELESS => VkFormat::BC5_UNORM_BLOCK,
    //         DxgiFormat::BC5_UNORM => VkFormat::BC5_UNORM_BLOCK,
    //         DxgiFormat::BC5_SNORM => VkFormat::BC5_SNORM_BLOCK,
    //         DxgiFormat::BC6H_TYPELESS => VkFormat::BC6H_UFLOAT_BLOCK,
    //         DxgiFormat::BC6H_UF16 => VkFormat::BC6H_UFLOAT_BLOCK,
    //         DxgiFormat::BC6H_SF16 => VkFormat::BC6H_SFLOAT_BLOCK,
    //         DxgiFormat::BC7_TYPELESS => VkFormat::BC7_UNORM_BLOCK,
    //         DxgiFormat::BC7_UNORM => VkFormat::BC7_UNORM_BLOCK,
    //         DxgiFormat::BC7_UNORM_SRGB => VkFormat::BC7_SRGB_BLOCK,
    //         DxgiFormat::B4G4R4A4_UNORM => VkFormat::A4R4G4B4_UNORM_PACK16,
    //         u => anyhow::bail!("Unsupported DXGI format conversion ({u:?} => ??)"),
    //     })
    // }

    pub fn bpp(&self) -> usize {
        match self {
            DxgiFormat::R32G32B32A32_TYPELESS
            | DxgiFormat::R32G32B32A32_FLOAT
            | DxgiFormat::R32G32B32A32_UINT
            | DxgiFormat::R32G32B32A32_SINT => 128,
            DxgiFormat::R32G32B32_TYPELESS
            | DxgiFormat::R32G32B32_FLOAT
            | DxgiFormat::R32G32B32_UINT
            | DxgiFormat::R32G32B32_SINT => 96,
            DxgiFormat::R16G16B16A16_TYPELESS
            | DxgiFormat::R16G16B16A16_FLOAT
            | DxgiFormat::R16G16B16A16_UNORM
            | DxgiFormat::R16G16B16A16_UINT
            | DxgiFormat::R16G16B16A16_SNORM
            | DxgiFormat::R16G16B16A16_SINT
            | DxgiFormat::R32G32_TYPELESS
            | DxgiFormat::R32G32_FLOAT
            | DxgiFormat::R32G32_UINT
            | DxgiFormat::R32G32_SINT
            | DxgiFormat::R32G8X24_TYPELESS
            | DxgiFormat::D32_FLOAT_S8X24_UINT
            | DxgiFormat::R32_FLOAT_X8X24_TYPELESS
            | DxgiFormat::X32_TYPELESS_G8X24_UINT
            | DxgiFormat::Y416
            | DxgiFormat::Y210
            | DxgiFormat::Y216 => 64,
            DxgiFormat::R10G10B10A2_TYPELESS
            | DxgiFormat::R10G10B10A2_UNORM
            | DxgiFormat::R10G10B10A2_UINT
            | DxgiFormat::R11G11B10_FLOAT
            | DxgiFormat::R8G8B8A8_TYPELESS
            | DxgiFormat::R8G8B8A8_UNORM
            | DxgiFormat::R8G8B8A8_UNORM_SRGB
            | DxgiFormat::R8G8B8A8_UINT
            | DxgiFormat::R8G8B8A8_SNORM
            | DxgiFormat::R8G8B8A8_SINT
            | DxgiFormat::R16G16_TYPELESS
            | DxgiFormat::R16G16_FLOAT
            | DxgiFormat::R16G16_UNORM
            | DxgiFormat::R16G16_UINT
            | DxgiFormat::R16G16_SNORM
            | DxgiFormat::R16G16_SINT
            | DxgiFormat::R32_TYPELESS
            | DxgiFormat::D32_FLOAT
            | DxgiFormat::R32_FLOAT
            | DxgiFormat::R32_UINT
            | DxgiFormat::R32_SINT
            | DxgiFormat::R24G8_TYPELESS
            | DxgiFormat::D24_UNORM_S8_UINT
            | DxgiFormat::R24_UNORM_X8_TYPELESS
            | DxgiFormat::X24_TYPELESS_G8_UINT
            | DxgiFormat::R9G9B9E5_SHAREDEXP
            | DxgiFormat::R8G8_B8G8_UNORM
            | DxgiFormat::G8R8_G8B8_UNORM
            | DxgiFormat::B8G8R8A8_UNORM
            | DxgiFormat::B8G8R8X8_UNORM
            | DxgiFormat::R10G10B10_XR_BIAS_A2_UNORM
            | DxgiFormat::B8G8R8A8_TYPELESS
            | DxgiFormat::B8G8R8A8_UNORM_SRGB
            | DxgiFormat::B8G8R8X8_TYPELESS
            | DxgiFormat::B8G8R8X8_UNORM_SRGB
            | DxgiFormat::AYUV
            | DxgiFormat::Y410
            | DxgiFormat::YUY2 => 32,
            DxgiFormat::P010 | DxgiFormat::P016 => 24,
            DxgiFormat::R8G8_TYPELESS
            | DxgiFormat::R8G8_UNORM
            | DxgiFormat::R8G8_UINT
            | DxgiFormat::R8G8_SNORM
            | DxgiFormat::R8G8_SINT
            | DxgiFormat::R16_TYPELESS
            | DxgiFormat::R16_FLOAT
            | DxgiFormat::D16_UNORM
            | DxgiFormat::R16_UNORM
            | DxgiFormat::R16_UINT
            | DxgiFormat::R16_SNORM
            | DxgiFormat::R16_SINT
            | DxgiFormat::B5G6R5_UNORM
            | DxgiFormat::B5G5R5A1_UNORM
            | DxgiFormat::A8P8
            | DxgiFormat::B4G4R4A4_UNORM => 16,
            DxgiFormat::NV12 | DxgiFormat::OPAQUE420 | DxgiFormat::NV11 => 12,
            DxgiFormat::R8_TYPELESS
            | DxgiFormat::R8_UNORM
            | DxgiFormat::R8_UINT
            | DxgiFormat::R8_SNORM
            | DxgiFormat::R8_SINT
            | DxgiFormat::A8_UNORM
            | DxgiFormat::AI44
            | DxgiFormat::IA44
            | DxgiFormat::P8 => 8,
            DxgiFormat::R1_UNORM => 1,
            DxgiFormat::BC1_TYPELESS
            | DxgiFormat::BC1_UNORM
            | DxgiFormat::BC1_UNORM_SRGB
            | DxgiFormat::BC4_TYPELESS
            | DxgiFormat::BC4_UNORM
            | DxgiFormat::BC4_SNORM => 4,
            DxgiFormat::BC2_TYPELESS
            | DxgiFormat::BC2_UNORM
            | DxgiFormat::BC2_UNORM_SRGB
            | DxgiFormat::BC3_TYPELESS
            | DxgiFormat::BC3_UNORM
            | DxgiFormat::BC3_UNORM_SRGB
            | DxgiFormat::BC5_TYPELESS
            | DxgiFormat::BC5_UNORM
            | DxgiFormat::BC5_SNORM
            | DxgiFormat::BC6H_TYPELESS
            | DxgiFormat::BC6H_UF16
            | DxgiFormat::BC6H_SF16
            | DxgiFormat::BC7_TYPELESS
            | DxgiFormat::BC7_UNORM
            | DxgiFormat::BC7_UNORM_SRGB => 8,
            u => panic!("{u:?}"),
        }
    }

    pub fn is_srgb(&self) -> bool {
        matches!(
            self,
            DxgiFormat::R8G8B8A8_UNORM_SRGB
                | DxgiFormat::BC1_UNORM_SRGB
                | DxgiFormat::BC2_UNORM_SRGB
                | DxgiFormat::BC3_UNORM_SRGB
                | DxgiFormat::B8G8R8A8_UNORM_SRGB
                | DxgiFormat::B8G8R8X8_UNORM_SRGB
                | DxgiFormat::BC7_UNORM_SRGB
        )
    }

    pub fn is_compressed(&self) -> bool {
        matches!(
            self,
            DxgiFormat::BC1_TYPELESS
                | DxgiFormat::BC1_UNORM
                | DxgiFormat::BC1_UNORM_SRGB
                | DxgiFormat::BC4_TYPELESS
                | DxgiFormat::BC4_UNORM
                | DxgiFormat::BC4_SNORM
                | DxgiFormat::BC2_TYPELESS
                | DxgiFormat::BC2_UNORM
                | DxgiFormat::BC2_UNORM_SRGB
                | DxgiFormat::BC3_TYPELESS
                | DxgiFormat::BC3_UNORM
                | DxgiFormat::BC3_UNORM_SRGB
                | DxgiFormat::BC5_TYPELESS
                | DxgiFormat::BC5_UNORM
                | DxgiFormat::BC5_SNORM
                | DxgiFormat::BC6H_TYPELESS
                | DxgiFormat::BC6H_UF16
                | DxgiFormat::BC6H_SF16
                | DxgiFormat::BC7_TYPELESS
                | DxgiFormat::BC7_UNORM
                | DxgiFormat::BC7_UNORM_SRGB
        )
    }

    pub fn calculate_pitch(&self, width: usize, height: usize) -> (usize, usize) {
        match *self {
            DxgiFormat::BC1_TYPELESS
            | DxgiFormat::BC1_UNORM
            | DxgiFormat::BC1_UNORM_SRGB
            | DxgiFormat::BC4_TYPELESS
            | DxgiFormat::BC4_UNORM
            | DxgiFormat::BC4_SNORM => {
                let nbw = ((width as i64 + 3) / 4).clamp(1, i64::MAX) as usize;
                let nbh = ((height as i64 + 3) / 4).clamp(1, i64::MAX) as usize;

                let pitch = nbw * 8;
                (pitch, pitch * nbh)
            }
            DxgiFormat::BC2_TYPELESS
            | DxgiFormat::BC2_UNORM
            | DxgiFormat::BC2_UNORM_SRGB
            | DxgiFormat::BC3_TYPELESS
            | DxgiFormat::BC3_UNORM
            | DxgiFormat::BC3_UNORM_SRGB
            | DxgiFormat::BC5_TYPELESS
            | DxgiFormat::BC5_UNORM
            | DxgiFormat::BC5_SNORM
            | DxgiFormat::BC6H_TYPELESS
            | DxgiFormat::BC6H_UF16
            | DxgiFormat::BC6H_SF16
            | DxgiFormat::BC7_TYPELESS
            | DxgiFormat::BC7_UNORM
            | DxgiFormat::BC7_UNORM_SRGB => {
                let nbw = ((width as i64 + 3) / 4).clamp(1, i64::MAX) as usize;
                let nbh = ((height as i64 + 3) / 4).clamp(1, i64::MAX) as usize;

                let pitch = nbw * 16;
                (pitch, pitch * nbh)
            }
            _ => {
                let pitch = (width * self.bpp() + 7) / 8;
                (pitch, height * pitch)
            }
        }
    }
}
