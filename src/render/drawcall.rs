use bitflags::bitflags;
use destiny_pkg::TagHash;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;

// #[bitfield(u64)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SortValue3d(u64);
//  {
// pub material: u32,

// #[bits(24)]
// pub depth: u32,

// #[bits(5)]
// pub special: SpecialDrawMode,

// #[bits(2)]
// pub transparency: Transparency,

// #[bits(1)]
// pub technique: ShadingTechnique,
// }

impl SortValue3d {
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn with_material(self, v: u32) -> Self {
        Self(self.0 | v as u64)
    }

    pub fn with_depth(self, v: u32) -> Self {
        Self(self.0 | (v as u64 & 0xffffff) << 32)
    }

    pub fn with_geometry_type(self, t: GeometryType) -> Self {
        Self(self.0 | (t.into_bits()) << 56)
    }

    pub fn with_shading_mode(self, t: ShadingMode) -> Self {
        Self(self.0 | (t.into_bits()) << 63)
    }

    pub fn with_transparency(self, t: Transparency) -> Self {
        Self(self.0 | (t.into_bits()) << 61)
    }

    pub fn material(&self) -> u32 {
        let this = self.0 & 0xffffffff;
        this as _
    }

    // pub const fn depth(&self) -> u32 {
    //     let this = (self.0 >> 32usize) & 0xffffff;
    //     this as _
    // }

    pub const fn geometry_type(&self) -> GeometryType {
        let this = (self.0 >> 56usize) & 0x1f;
        GeometryType::from_bits(this)
    }

    pub const fn shading_mode(&self) -> ShadingMode {
        let this = (self.0 >> 63usize) & 0x1;
        ShadingMode::from_bits(this)
    }

    pub const fn transparency(&self) -> Transparency {
        let this = (self.0 >> 61usize) & 0x3;
        Transparency::from_bits(this)
    }
}

#[repr(u64)]
#[derive(Debug, PartialEq, Eq)]
pub enum GeometryType {
    Static = 0,
    StaticDecal = 1,
    Terrain = 2,
    Entity = 3,
    // Decal = 4,
}

impl GeometryType {
    const fn into_bits(self) -> u64 {
        self as _
    }

    const fn from_bits(value: u64) -> Self {
        match value {
            0 => Self::Static,
            1 => Self::StaticDecal,
            2 => Self::Terrain,
            3 => Self::Entity,
            // 4 => Self::Decal,
            _ => Self::Static,
        }
    }
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

    pub fn should_write_depth(&self) -> bool {
        matches!(self, Self::None | Self::Cutout)
    }
}

#[repr(u64)]
#[derive(Debug, PartialEq)]
pub enum ShadingMode {
    Deferred = 0,
    Forward = 1,
}

impl ShadingMode {
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
    #[derive(Copy, Clone)]
    pub struct ShaderStages: u8 {
        const VERTEX = (1 << 0);
        const PIXEL = (1 << 1);
        const COMPUTE = (1 << 2);
        const GEOMETRY = (1 << 3);

        const SHADING = Self::VERTEX.bits() | Self::PIXEL.bits();
    }
}

// TODO(cohae): Can be crammed into 32 bits?
#[derive(Clone)]
pub struct ConstantBufferBinding {
    pub buffer: ID3D11Buffer, // at least 25 bits for a hash
    pub slot: u32,            // 4 bits
                              // pub stages: ShaderStages, // 2 bits
                              // Total: 31 (+1 more bit, put that in buffer hash)
}

impl ConstantBufferBinding {
    pub fn new(slot: u32, buffer: ID3D11Buffer) -> ConstantBufferBinding {
        ConstantBufferBinding { buffer, slot }
    }
}

#[derive(Clone)]
pub struct DrawCall {
    pub vertex_buffers: Vec<TagHash>,
    pub index_buffer: TagHash,
    pub color_buffer: Option<TagHash>,
    pub input_layout_hash: u64,

    // TODO(cohae): Will this be used for anything other than instances/rigid_model? Can just be a pointer or an id or whatevs otherwise
    pub buffer_bindings: Vec<ConstantBufferBinding>,
    // pub cb11: Option<ID3D11Buffer>,
    /// Applied on top of the base material
    pub variant_material: Option<TagHash>,
    pub dyemap: Option<TagHash>,

    pub index_start: u32,
    pub index_count: u32,
    pub instance_start: Option<u32>,
    pub instance_count: Option<u32>,
    pub primitive_type: D3D_PRIMITIVE_TOPOLOGY,
}
