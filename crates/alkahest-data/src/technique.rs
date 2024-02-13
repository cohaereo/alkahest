use destiny_pkg::TagHash;
use tiger_parse::tiger_tag;

use crate::{tfx::TfxShaderStage, ExtendedHash};

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff)]
pub struct STechnique {
    pub file_size: u64,
    /// 0 = ???
    /// 1 = normal
    /// 2 = depth prepass?
    /// 6 = ????????
    pub unk8: u32,
    pub unkc: u32,
    pub unk10: u32,
    pub unk14: u32,
    pub unk18: u32,
    pub unk1c: u32,
    pub unk20: u16,
    pub unk22: u16,
    pub unk24: u32,
    pub unk28: u32,
    pub unk2c: u32,
    pub unk30: [u32; 16],

    pub shader_vertex: STechniqueShader,
    pub shader_unk1: STechniqueShader,
    pub shader_unk2: STechniqueShader,
    pub shader_unk3: STechniqueShader,
    pub shader_pixel: STechniqueShader,
    pub shader_compute: STechniqueShader,
}

impl STechnique {
    pub fn all_shaders(&self) -> Vec<(TfxShaderStage, &STechniqueShader)> {
        vec![
            (TfxShaderStage::Vertex, &self.shader_vertex),
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

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff)]
pub struct STechniqueShader {
    pub shader: TagHash,
    pub unk4: u32,
    pub textures: Vec<SMaterialTextureAssignment>, // 0x8
    pub unk18: u64,
    pub bytecode: Vec<u8>,                   // 0x20
    pub bytecode_constants: Vec<glam::Vec4>, // 0x30
    pub samplers: Vec<ExtendedHash>,         // 0x40
    pub unk50: Vec<glam::Vec4>,              // 0x50

    pub unk60: [u32; 4], // 0x60

    pub constant_buffer_slot: u32, // 0x70
    pub constant_buffer: TagHash,  // 0x74

    pub unk78: [u32; 6],
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0xffffffff)]
pub struct SMaterialTextureAssignment {
    /// Material slot to assign to
    pub slot: u32,
    _pad: u32,
    pub texture: ExtendedHash,
}
