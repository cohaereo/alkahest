use std::{
    fmt::Debug,
    io::{Read, Seek},
};

use glam::Vec4;
use int_enum::IntEnum;
use tiger_parse::{tiger_type, Endian, Padding, TigerReadable};
use tiger_pkg::TagHash;

use crate::{tag::WideHash, tfx::enums::ShaderStage};

#[derive(Clone)]
#[tiger_type(id = 0x80806DAA)]
pub struct STechnique {
    pub file_size: u64,
    pub bind_mode: TechniqueBindMode,
    pub unkc: u32,
    pub unk10: u32,
    pub unk14: u32,
    pub unk18: u32,
    pub unk1c: u32,

    pub used_scopes: TfxScopeBits,
    pub compatible_scopes: TfxScopeBits,

    pub states: PipelineState,
    pub unk34: [u32; 15],

    pub shader_vertex: STechniqueShader,
    pub shader_hull: STechniqueShader,
    pub shader_domain: STechniqueShader,
    pub shader_geometry: STechniqueShader,
    pub shader_pixel: STechniqueShader,
    pub shader_compute: STechniqueShader,
}

impl STechnique {
    pub fn all_shaders(&self) -> Vec<(ShaderStage, &STechniqueShader)> {
        vec![
            (ShaderStage::Vertex, &self.shader_vertex),
            (ShaderStage::Geometry, &self.shader_geometry),
            (ShaderStage::Hull, &self.shader_hull),
            (ShaderStage::Domain, &self.shader_domain),
            (ShaderStage::Pixel, &self.shader_pixel),
            (ShaderStage::Compute, &self.shader_compute),
        ]
    }

    pub fn all_valid_shaders(&self) -> Vec<(ShaderStage, &STechniqueShader)> {
        self.all_shaders()
            .into_iter()
            .filter(|(_, s)| s.shader.is_some())
            .collect()
    }
}

/// Indicates what to bind
///     VertexPixel - bind vs+ps, unbind gs+hs+ds+cs (also does stuff with gear_dye scopes, hasn't been reversed yet)
///     VertexOnly - bind vs, unbind ps+gs+hs+ds+cs
///     VertexGeometryPixel - bind vs+gs+ps, unbind hs+ds+cs
///     VertexPixelTesselated - bind vs+hs+ds+ps, unbind gs+cs
///     VertexOnlyTesselated - bind vs+hs+ds, unbind ps+cs+gs
///     Compute - bind cs, unbind vs+gs+hs+ds+ps
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, IntEnum)]
pub enum TechniqueBindMode {
    VertexPixel = 1,
    VertexOnly = 2,
    VertexGeometryPixel = 3,
    VertexPixelTesselated = 4,
    VertexOnlyTesselated = 5,
    Compute = 6,
}

impl TigerReadable for TechniqueBindMode {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let v = u32::read_ds_endian(reader, endian)?;
        Self::try_from(v).map_err(|_| tiger_parse::Error::EnumVariantOutOfRange(v as usize))
    }

    const SIZE: usize = 4;
}

#[derive(Clone)]
#[tiger_type(size = 0x90)]
pub struct STechniqueShader {
    pub shader: TagHash,
    pub unk4: u32,
    pub constants: SDynamicConstants,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x80806DCF)]
pub struct SMaterialTextureAssignment {
    /// Material slot to assign to
    pub slot: u32,
    _pad: Padding<4>,
    pub texture: WideHash,
}

#[derive(Clone)]
#[tiger_type(size = 0x80)]
pub struct SDynamicConstants {
    pub textures: Vec<SMaterialTextureAssignment>,
    pub unk10: u64,
    pub bytecode: Vec<u8>,                // 0x18
    pub bytecode_constants: Vec<Vec4>,    // 0x28
    pub samplers: Vec<SSamplerReference>, // 0x38
    pub unk30: Vec<Vec4>,                 // 0x48
    pub unk40: [u32; 4],                  // 0x58

    pub constant_buffer_slot: i32, // 0x68
    pub constant_buffer: TagHash,  // 0x6c
}

#[derive(Clone)]
#[tiger_type(id = 0x8080013F)]
pub struct SSamplerReference {
    pub sampler: TagHash,
    pub unk4: u32,
    pub unk8: u32,
    pub unkc: u32,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct TfxScopeBits: u64 {
        const FRAME                        = 1 << 0;
        const VIEW                         = 1 << 1;
        const RIGID_MODEL                  = 1 << 2;
        const EDITOR_MESH                  = 1 << 3;
        const EDITOR_TERRAIN               = 1 << 4;
        const CUI_VIEW                     = 1 << 5;
        const CUI_OBJECT                   = 1 << 6;
        const SKINNING                     = 1 << 7;
        const SPEEDTREE                    = 1 << 8;
        const CHUNK_MODEL                  = 1 << 9;
        const DECAL                        = 1 << 10;
        const INSTANCES                    = 1 << 11;
        const SPEEDTREE_LOD_DRAWCALL_DATA  = 1 << 12;
        const TRANSPARENT                  = 1 << 13;
        const TRANSPARENT_ADVANCED         = 1 << 14;
        const SDSM_BIAS_AND_SCALE_TEXTURES = 1 << 15;
        const POSTPROCESS                  = 1 << 16;
        const CUI_BITMAP                   = 1 << 17;
        const CUI_STANDARD                 = 1 << 18;
        const UI_FONT                      = 1 << 19;
        const CUI_HUD                      = 1 << 20;
        const PARTICLE_TRANSFORMS          = 1 << 21;
        const PARTICLE_LOCATION_METADATA   = 1 << 22;
        const CUBEMAP_VOLUME               = 1 << 23;
        const GEAR_PLATED_TEXTURES         = 1 << 24;
        const GEAR_DYE_0                   = 1 << 25;
        const GEAR_DYE_1                   = 1 << 26;
        const GEAR_DYE_2                   = 1 << 27;
        const GEAR_DYE_DECAL               = 1 << 28;
        const GENERIC_ARRAY                = 1 << 29;
        const WEATHER                      = 1 << 30;
    }
}

// TODO(cohae): tiger-parse doesnt work with bitflags, so we have to implement this manually
impl TigerReadable for TfxScopeBits {
    fn read_ds_endian<R: Read + Seek>(reader: &mut R, endian: Endian) -> tiger_parse::Result<Self> {
        let bits: u64 = u64::read_ds_endian(reader, endian)?;
        Ok(Self::from_bits_truncate(bits))
    }

    const SIZE: usize = 8;
}

/// Current indices for blend, rasterizer, depth bias and depth stencil states
#[tiger_type(size = 4)]
#[derive(Clone, Copy, Default)]
pub struct PipelineState {
    blend_state: u8,
    depth_stencil_state: u8,
    rasterizer_state: u8,
    depth_bias_state: u8,
}

impl PipelineState {
    pub fn new(
        blend_state: Option<usize>,
        depth_stencil_state: Option<usize>,
        rasterizer_state: Option<usize>,
        depth_bias_state: Option<usize>,
    ) -> Self {
        Self {
            blend_state: blend_state.map(|v| v | 0x80).unwrap_or(0) as u8,
            depth_stencil_state: depth_stencil_state.map(|v| v | 0x80).unwrap_or(0) as u8,
            rasterizer_state: rasterizer_state.map(|v| v | 0x80).unwrap_or(0) as u8,
            depth_bias_state: depth_bias_state.map(|v| v | 0x80).unwrap_or(0) as u8,
        }
    }

    #[inline(always)]
    pub fn from_raw(raw: u32) -> Self {
        Self {
            blend_state: (raw & 0xff) as u8,
            depth_stencil_state: ((raw >> 8) & 0xff) as u8,
            rasterizer_state: ((raw >> 16) & 0xff) as u8,
            depth_bias_state: ((raw >> 24) & 0xff) as u8,
        }
    }

    #[inline(always)]
    pub fn raw(&self) -> u32 {
        (self.blend_state as u32)
            | ((self.depth_stencil_state as u32) << 8)
            | ((self.rasterizer_state as u32) << 16)
            | ((self.depth_bias_state as u32) << 24)
    }

    /// Creates a new selection, filling unset states in `other` with the default state in `self`
    pub fn select(&self, other: &PipelineState) -> PipelineState {
        let current = self.raw();
        let other = other.raw();
        let new_states = ((other >> 7 & 0x1010101) * 0xff) & (current ^ other) ^ current;

        PipelineState::from_raw(new_states)

        // PipelineState::new(
        //     other.blend_state().or_else(|| self.blend_state()),
        //     other
        //         .depth_stencil_state()
        //         .or_else(|| self.depth_stencil_state()),
        //     other.rasterizer_state().or_else(|| self.rasterizer_state()),
        //     other.depth_bias_state().or_else(|| self.depth_bias_state()),
        // )
    }

    pub fn blend_state(&self) -> Option<usize> {
        if self.blend_state & 0x80 != 0 {
            Some((self.blend_state & 0x7f) as usize)
        } else {
            None
        }
    }

    pub fn depth_stencil_state(&self) -> Option<usize> {
        if self.depth_stencil_state & 0x80 != 0 {
            Some((self.depth_stencil_state & 0x7f) as usize)
        } else {
            None
        }
    }

    pub fn rasterizer_state(&self) -> Option<usize> {
        if self.rasterizer_state & 0x80 != 0 {
            Some((self.rasterizer_state & 0x7f) as usize)
        } else {
            None
        }
    }

    pub fn depth_bias_state(&self) -> Option<usize> {
        if self.depth_bias_state & 0x80 != 0 {
            Some((self.depth_bias_state & 0x7f) as usize)
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.blend_state = 0;
        self.depth_stencil_state = 0;
        self.rasterizer_state = 0;
        self.depth_bias_state = 0;
    }
}

impl Debug for PipelineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateSelection")
            .field("blend_state", &self.blend_state())
            .field("depth_stencil_state", &self.depth_stencil_state())
            .field("rasterizer_state", &self.rasterizer_state())
            .field("depth_bias_state", &self.depth_bias_state())
            .finish()
    }
}
