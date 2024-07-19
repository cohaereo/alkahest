use std::{
    fmt::Debug,
    io::{Read, Seek},
};

use destiny_pkg::TagHash;
use glam::Vec4;
use tiger_parse::{tiger_tag, Endian, NullString, Pointer, TigerReadable};

use crate::{tfx::TfxShaderStage, WideHash};

#[derive(Clone)]
#[tiger_tag(id = 0x808071E8)]
pub struct STechnique {
    pub file_size: u64,
    /// Indicates what to bind
    ///     1 - bind vs+ps, unbind gs+hs+ds+cs (also does stuff with gear_dye scopes)
    ///     2 - bind vs, unbind ps+gs+hs+ds+cs
    ///     3 - bind vs+gs+ps, unbind hs+ds+cs
    ///     4 - bind vs+hs+ds+ps, unbind gs+cs
    ///     5 - bind vs+hs+ds, unbind ps+cs+gs
    ///     6 - bind cs, unbind vs+gs+hs+ds+ps
    pub unk8: u32,
    pub unkc: u32,
    pub unk10: u32,
    pub unk14: u32,

    pub used_scopes: TfxScopeBits,
    pub compatible_scopes: TfxScopeBits,

    pub unk20: u32,
    pub states: StateSelection,
    pub unk28: [u32; 8],

    // 0x48
    pub shader_vertex: STechniqueShader,
    pub shader_unk1: STechniqueShader,
    pub shader_unk2: STechniqueShader,
    pub shader_geometry: STechniqueShader,
    // 0x2c8
    pub shader_pixel: STechniqueShader,
    pub shader_compute: STechniqueShader,
}

impl STechnique {
    pub fn all_shaders(&self) -> Vec<(TfxShaderStage, &STechniqueShader)> {
        vec![
            (TfxShaderStage::Vertex, &self.shader_vertex),
            (TfxShaderStage::Geometry, &self.shader_geometry),
            (TfxShaderStage::Pixel, &self.shader_pixel),
            (TfxShaderStage::Compute, &self.shader_compute),
        ]
    }

    pub fn all_valid_shaders(&self) -> Vec<(TfxShaderStage, &STechniqueShader)> {
        self.all_shaders()
            .into_iter()
            .filter(|(_, s)| s.shader.is_some())
            .collect()
    }
}

#[derive(Clone)]
#[tiger_tag(id = 0xffffffff, size = 0xa0)]
pub struct STechniqueShader {
    pub shader: TagHash,
    pub unk4: u32,
    pub textures: Vec<SMaterialTextureAssignment>, // 0x8
    pub unk18: u64,
    pub constants: SDynamicConstants,

    pub unk78: [u32; 6],
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80807211)]
pub struct SMaterialTextureAssignment {
    /// Material slot to assign to
    pub slot: u32,
    pub texture: TagHash,
}

#[derive(Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80806cb1 {
    pub file_size: u64,
    pub unk8: TagHash,
    pub unkc: u32,
    pub unk10: Vec<Unk80806cb6>,
    pub unk20: Vec<Unk80806cb5>,
    pub unk30: TagHash,
    pub unk34: TagHash,
    pub unk38: TagHash,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80806cb5 {
    pub name: Pointer<NullString>,
    pub unk8: u32,
    pub unkc: TagHash,
}

pub type Unk80806cb6 = Unk80806cb5;

#[derive(Clone)]
#[tiger_tag(size = 0x68)]
pub struct SDynamicConstants {
    pub bytecode: Vec<u8>,
    pub bytecode_constants: Vec<Vec4>,
    pub samplers: Vec<SSamplerReference>,
    pub unk30: Vec<Vec4>,
    pub unk40: [u32; 8],

    pub constant_buffer_slot: i32, // 0x60
    pub constant_buffer: TagHash,
}

#[derive(Clone)]
#[tiger_tag(id = 0x808073F3)]
pub struct SSamplerReference {
    pub sampler: TagHash,
    pub unk4: u32,
    pub unk8: u32,
    pub unkc: u32,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct TfxScopeBits: u32 {
        const FRAME                             = 1 << 0;
        const VIEW                              = 1 << 1;
        const RIGID_MODEL                       = 1 << 2;
        const EDITOR_MESH                       = 1 << 3;
        const EDITOR_TERRAIN                    = 1 << 4;
        const CUI_VIEW                          = 1 << 5;
        const CUI_OBJECT                        = 1 << 6;
        const SKINNING                          = 1 << 7;
        const SPEEDTREE                         = 1 << 8;
        const CHUNK_MODEL                       = 1 << 9;
        const DECAL                             = 1 << 10;
        const INSTANCES                         = 1 << 11;
        const SPEEDTREE_LOD_DRAWCALL_DATA       = 1 << 12;
        const TRANSPARENT                       = 1 << 13;
        const TRANSPARENT_ADVANCED              = 1 << 14;
        const SDSM_BIAS_AND_SCALE_TEXTURES      = 1 << 15;
        const TERRAIN                           = 1 << 16;
        const POSTPROCESS                       = 1 << 17;
        const CUI_BITMAP                        = 1 << 18;
        const CUI_STANDARD                      = 1 << 19;
        const UI_FONT                           = 1 << 20;
        const CUI_HUD                           = 1 << 21;
        const PARTICLE_TRANSFORMS               = 1 << 22;
        const PARTICLE_LOCATION_METADATA        = 1 << 23;
        const CUBEMAP_VOLUME                    = 1 << 24;
        const GEAR_PLATED_TEXTURES              = 1 << 25;
        const GEAR_DYE_0                        = 1 << 26;
        const GEAR_DYE_1                        = 1 << 27;
        const GEAR_DYE_2                        = 1 << 28;
        const GEAR_DYE_DECAL                    = 1 << 29;
        const GENERIC_ARRAY                     = 1 << 30;
        const WEATHER                           = 1 << 31;
    }
}

// TODO(cohae): tiger-parse doesnt work with bitflags, so we have to implement this manually
impl TigerReadable for TfxScopeBits {
    fn read_ds_endian<R: Read + Seek>(reader: &mut R, endian: Endian) -> tiger_parse::Result<Self> {
        let bits: u32 = u32::read_ds_endian(reader, endian)?;
        Ok(Self::from_bits_truncate(bits))
    }

    const ZEROCOPY: bool = true;
    const SIZE: usize = 8;
}

/// Selection of blend, rasterizer, depth bias and depth stencil state
#[tiger_tag(size = 4)]
#[derive(Clone, Copy)]
pub struct StateSelection {
    /// Value is encoded as 0xDDCCBBAA
    /// Where each byte specifies a state, using the high bit to indicate if the state is set
    /// A - Blend state
    /// B - Depth stencil state
    /// C - Rasterizer state
    /// D - Depth bias state
    inner: u32,
}

impl StateSelection {
    pub fn new(
        blend_state: Option<usize>,
        depth_stencil_state: Option<usize>,
        rasterizer_state: Option<usize>,
        depth_bias_state: Option<usize>,
    ) -> Self {
        let mut inner = 0;

        inner |= 0x80 | blend_state.unwrap_or(0) as u32;
        inner |= (0x80 | depth_stencil_state.unwrap_or(0) as u32) << 8;
        inner |= (0x80 | rasterizer_state.unwrap_or(0) as u32) << 16;
        inner |= (0x80 | depth_bias_state.unwrap_or(0) as u32) << 24;

        Self { inner }
    }

    #[inline(always)]
    pub fn from_raw(raw: u32) -> Self {
        Self { inner: raw }
    }

    #[inline(always)]
    pub fn raw(&self) -> u32 {
        self.inner
    }

    /// Creates a new selection, filling unset states in `other` with the default state in `self`
    pub fn select(&self, other: &StateSelection) -> StateSelection {
        let new_states =
            ((other.raw() >> 7 & 0x1010101) * 0xff) & (self.raw() ^ other.raw()) ^ self.raw();

        StateSelection::from_raw(new_states)
    }

    pub fn blend_state(&self) -> Option<usize> {
        if self.inner & 0x80 != 0 {
            Some((self.inner & 0x7f) as usize)
        } else {
            None
        }
    }

    pub fn depth_stencil_state(&self) -> Option<usize> {
        let v = (self.inner >> 8) & 0xff;
        if v & 0x80 != 0 {
            Some((v & 0x7f) as usize)
        } else {
            None
        }
    }

    pub fn rasterizer_state(&self) -> Option<usize> {
        let v = (self.inner >> 16) & 0xff;
        if v & 0x80 != 0 {
            Some((v & 0x7f) as usize)
        } else {
            None
        }
    }

    pub fn depth_bias_state(&self) -> Option<usize> {
        let v = (self.inner >> 24) & 0xff;
        if v & 0x80 != 0 {
            Some((v & 0x7f) as usize)
        } else {
            None
        }
    }
}

impl Debug for StateSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateSelection")
            .field("blend_state", &self.blend_state())
            .field("depth_stencil_state", &self.depth_stencil_state())
            .field("rasterizer_state", &self.rasterizer_state())
            .field("depth_bias_state", &self.depth_bias_state())
            .finish()
    }
}
