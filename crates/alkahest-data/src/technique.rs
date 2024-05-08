use std::fmt::Debug;

use destiny_pkg::TagHash;
use tiger_parse::{tiger_tag, NullString, Pointer};

use crate::{tfx::TfxShaderStage, WideHash};

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806DAA)]
pub struct STechnique {
    pub file_size: u64,
    /// Indicates what to bind
    ///
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
    pub unk18: u32,
    pub unk1c: u32,

    pub used_scopes: u64,
    pub compatible_scopes: u64,

    pub states: StateSelection,
    pub unk34: [u32; 15],

    pub shader_vertex: STechniqueShader,
    pub shader_unk1: STechniqueShader,
    pub shader_unk2: STechniqueShader,
    pub shader_geometry: STechniqueShader,
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

    //     pub fn debug_header_string(&self) -> String {
    //         format!(
    //             "STechnique {{
    //     unk8: 0x{:x},
    //     unkc: 0x{:x},
    //     unk10: 0x{:x},
    //     unk14: 0x{:x},
    //     unk18: 0x{:x},
    //     unk1c: 0x{:x},
    //     unk20: 0x{:x},
    //     unk22: 0x{:x},
    //     unk24: 0x{:x},
    //     unk28: 0x{:x},
    //     unk2c: 0x{:x},
    //     states: 0x{:?},
    //     unk34: {:x?}
    // }}",
    //             self.unk8,
    //             self.unkc,
    //             self.unk10,
    //             self.unk14,
    //             self.unk18,
    //             self.unk1c,
    //             self.unk20,
    //             self.unk22,
    //             self.unk24,
    //             self.unk28,
    //             self.unk2c,
    //             self.states,
    //             self.unk34
    //         )
    //     }
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff)]
pub struct STechniqueShader {
    pub shader: TagHash,
    pub unk4: u32,
    pub textures: Vec<SMaterialTextureAssignment>, // 0x8
    pub unk18: u64,
    pub bytecode: Vec<u8>,                   // 0x20
    pub bytecode_constants: Vec<glam::Vec4>, // 0x30
    pub samplers: Vec<WideHash>,             // 0x40
    pub unk50: Vec<glam::Vec4>,              // 0x50

    pub unk60: [u32; 4], // 0x60

    pub constant_buffer_slot: i32, // 0x70
    pub constant_buffer: TagHash,  // 0x74

    pub unk78: [u32; 6],
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x80806DCF)]
pub struct SMaterialTextureAssignment {
    /// Material slot to assign to
    pub slot: u32,
    _pad: u32,
    pub texture: WideHash,
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

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80806da1 {
    pub file_size: u64,
    pub unk8: u64,
    pub unk10: [u32; 8],

    pub bytecode: Vec<u8>,
    pub bytecode_constants: Vec<glam::Vec4>,
    pub unk50: [u32; 4],
    pub unk60: Vec<glam::Vec4>,
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
