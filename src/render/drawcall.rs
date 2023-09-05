use bitfield_struct::bitfield;
use bitflags::bitflags;
use destiny_pkg::TagHash;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;

#[bitfield(u64)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct SortValue3d {
    pub material: u32,

    #[bits(24)]
    pub depth: u32,

    #[bits(2)]
    pub transparency: Transparency,

    #[bits(1)]
    pub technique: ShadingTechnique,

    #[bits(5)]
    __: u8,
}

#[repr(u64)]
#[derive(Debug, PartialEq, Eq)]
pub enum Transparency {
    None = 0,
    Cutout = 1,
    Blend = 2,
    Additive = 3,
}

impl Transparency {
    const fn into_bits(self) -> u64 {
        self as _
    }

    const fn from_bits(value: u64) -> Self {
        match value {
            0 => Self::None,
            1 => Self::Cutout,
            2 => Self::Blend,
            3 => Self::Additive,
            _ => Self::None,
        }
    }
}

#[repr(u64)]
#[derive(Debug, PartialEq)]
pub enum ShadingTechnique {
    Deferred = 0,
    Forward = 1,
}

impl ShadingTechnique {
    const fn into_bits(self) -> u64 {
        self as _
    }

    const fn from_bits(value: u64) -> Self {
        match value {
            0 => Self::Deferred,
            1 => Self::Forward,
            _ => Self::Deferred,
        }
    }
}

bitflags! {
    pub struct ShaderStages: u8 {
        const VERTEX = (1 << 0);
        const PIXEL = (1 << 1);
        const COMPUTE = (1 << 2);
    }
}

// // TODO(cohae): Can be crammed into 32 bits?
// pub struct ConstantBufferBinding {
//     pub buffer: ID3D11Buffer, // at least 25 bits for a hash
//     pub slot: u8,             // 4 bits
//     pub stages: ShaderStages, // 2 bits
//                               // Total: 31 (+1 more bit, put that in buffer hash)
// }

#[derive(Clone)]
pub struct DrawCall {
    // TODO: Get these from render data
    pub vertex_buffer: ID3D11Buffer,
    pub vertex_buffer_stride: u32,
    pub index_buffer: ID3D11Buffer,
    pub index_format: DXGI_FORMAT,

    // TODO(cohae): Will this be used for anything other than instances/rigid_model? Can just be a pointer or an id or whatevs otherwise
    // pub buffer_bindings: Vec<ConstantBufferBinding>,
    pub cb11: Option<ID3D11Buffer>,

    /// Applied on top of the base material
    pub variant_material: Option<TagHash>,

    pub index_start: u32,
    pub index_count: u32,
    pub instance_start: Option<u32>,
    pub instance_count: Option<u32>,
    pub primitive_type: D3D_PRIMITIVE_TOPOLOGY,
}
