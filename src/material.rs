use std::io::SeekFrom;
use std::ops::Deref;

use crate::render::drawcall::ShaderStages;
use crate::render::renderer::Renderer;
use crate::render::{DeviceContextSwapchain, RenderData};
use crate::structure::{ExtendedHash, RelPointer, TablePointer};
use crate::types::Vector4;
use binrw::{BinRead, NullString};
use destiny_pkg::TagHash;
use egui::epaint::ahash::{HashMap, HashMapExt};
use crate::material_shader::{MaterialShader, ShaderType};

#[derive(BinRead, Debug, Clone)]
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
    pub unk30: [u32; 6],

    #[br(seek_before(SeekFrom::Start(0x70)))]
    pub vertex_shader: TagHash,
    pub unk5c: u32,
    pub vs_textures: TablePointer<SMaterialTextureAssignment>,
    pub unk70: u64,
    pub vs_bytecode: TablePointer<u8>,
    pub vs_bytecode_constants: TablePointer<Vector4>,
    pub vs_samplers: TablePointer<ExtendedHash>,
    pub unka8: TablePointer<Vector4>,
    pub unkb8: [u32; 9],

    #[br(seek_before(SeekFrom::Start(0xe4)))]
    pub unke4: TagHash,

    // pub unke0: [u32; 126],
    #[br(seek_before(SeekFrom::Start(0x2b0)))]
    pub pixel_shader: TagHash,
    pub unk2b4: u32,
    pub ps_textures: TablePointer<SMaterialTextureAssignment>,
    pub unk2c8: u64,
    pub ps_bytecode: TablePointer<u8>,
    pub ps_bytecode_constants: TablePointer<Vector4>,
    pub ps_samplers: TablePointer<ExtendedHash>,
    pub unk2f8: TablePointer<Vector4>,
    // pub unk2f8: [u32; 9],
    /// Pointer to a float4 buffer, usually passed into cbuffer0
    #[br(seek_before(SeekFrom::Start(0x324)))]
    pub unk334: TagHash,
}

#[derive(BinRead, Debug, Clone)]
pub struct SMaterialTextureAssignment {
    /// Material slot to assign to
    pub index: u32,
    _pad: u32,
    pub texture: ExtendedHash,
}

// #[derive(BinRead, Debug, Clone)]
// pub struct Unk808073f3 {
//     pub sampler: TagHash64,
//     pub unk8: u32,
//     pub unkc: u32,
// }

pub struct Technique {
    pub technique: STechnique,
    pub shaders: HashMap<ShaderType, MaterialShader>,
    pub textures: HashMap<ShaderType, TablePointer<SMaterialTextureAssignment>>,

    hash: TagHash
}

impl Technique {

    // TODO(cohae): load_shaders is a hack, i fucking hate locks
    pub fn load(material: STechnique, hash: TagHash) -> Self {
        let _span = debug_span!("Load material", hash = %hash).entered();

        let mut shaders = HashMap::new();
        shaders.insert(ShaderType::Vertex, MaterialShader::create(ShaderType::Vertex, &material));
        shaders.insert(ShaderType::Pixel, MaterialShader::create(ShaderType::Pixel, &material));

        let mut textures = HashMap::new();
        textures.insert(ShaderType::Vertex, material.vs_textures.clone());
        textures.insert(ShaderType::Pixel, material.ps_textures.clone());

        Self {
            technique: material,
            shaders,
            textures,
            hash
        }
    }

    pub fn load_bindable(material: STechnique, hash: TagHash, renderer: &Renderer, load_shaders: bool) -> Self {
        let mut mat = Technique::load(material, hash);

        for (shader_type, shader) in mat.shaders.iter_mut() {
            shader.setup_material_scope_buffer(renderer);
            if load_shaders {
                if shader_type == &ShaderType::Vertex {
                    renderer.render_data.load_vshader(&renderer.dcs, shader.shader);
                } else if shader_type == &ShaderType::Pixel {
                    renderer.render_data.load_pshader(&renderer.dcs, shader.shader);
                }
            }
        }

        return mat;
    }

    // pub fn tag(&self) -> TagHash {
    //     self.tag
    // }

    pub fn bind(
        &self,
        renderer: &Renderer,
        render_data: &RenderData,
        _stages: ShaderStages
    ) -> anyhow::Result<()> {
        for (shader_type, shader) in self.shaders.iter() {
            shader.bind(renderer, render_data, self.textures.get(&shader_type).unwrap());
        }

        Ok(())
    }

    pub fn evaluate_bytecode(&mut self, renderer: &Renderer, shader_type: ShaderType) {
        let shader = match self.shaders.get_mut(&shader_type) {
            Some(s) => s,
            None => { return; }
        };

        if let Some(ref material_scope ) = &shader.material_scope_buffer {
            if let Some(ref mut tfx) = shader.tfx {
                let t = shader_type.to_string();
                let _span = info_span!("Evaluating TFX bytecode of material {} for the {}", t).entered();
                let res = tfx.0.evaluate(
                    renderer,
                    material_scope,
                    tfx.1.as_slice()
                );
                if let Err(e) = res {
                    error!( "TFX bytecode evaluation failed for {}, disabling: {e}", shader_type.to_string());
                    //TODO: Dump TFX Interpreter Stats.
                }
            }
        }
    }

    pub fn unbind_textures(&self, dcs: &DeviceContextSwapchain) {
        unsafe {
            for p in &self.vs_textures {
                dcs.context().VSSetShaderResources(p.index, Some(&[None]));
            }

            for p in &self.ps_textures {
                dcs.context().PSSetShaderResources(p.index, Some(&[None]));
            }
        }
    }
}

impl Deref for Technique {
    type Target = STechnique;

    fn deref(&self) -> &Self::Target {
        &self.technique
    }
}
#[derive(BinRead, Debug)]
pub struct Unk80806cb1 {
    pub file_size: u64,
    pub unk8: TagHash,
    pub unkc: u32,
    pub unk10: TablePointer<Unk80806cb6>,
    pub unk20: TablePointer<Unk80806cb5>,
    pub unk30: TagHash,
    pub unk34: TagHash,
    pub unk38: TagHash,
}

#[derive(BinRead, Debug)]
pub struct Unk80806cb5 {
    pub name: RelPointer<NullString>,
    pub unk8: u32,
    pub unkc: TagHash,
}

pub type Unk80806cb6 = Unk80806cb5;
