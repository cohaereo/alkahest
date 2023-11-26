use std::ops::Deref;

use crate::packages::package_manager;
use crate::render::bytecode::externs::TfxShaderStage;
use crate::render::bytecode::interpreter::TfxBytecodeInterpreter;
use crate::render::bytecode::opcodes::TfxBytecodeOp;
use crate::render::cbuffer::ConstantBufferCached;
use crate::render::drawcall::ShaderStages;
use crate::render::renderer::Renderer;
use crate::render::{DeviceContextSwapchain, RenderData};
use crate::structure::ExtendedHash;
use crate::structure::{RelPointer, TablePointer};
use crate::types::Vector4;
use crate::util::RwLock;
use binrw::{BinRead, NullString};
use destiny_pkg::TagHash;
use glam::Vec4;
use itertools::Itertools;

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
            .collect_vec()
    }
}

#[derive(BinRead, Debug, Clone)]
pub struct STechniqueShader {
    pub shader: TagHash,
    pub unk4: u32,
    pub textures: TablePointer<SMaterialTextureAssignment>, // 0x8
    pub unk18: u64,
    pub bytecode: TablePointer<u8>,                // 0x20
    pub bytecode_constants: TablePointer<Vector4>, // 0x30
    pub samplers: TablePointer<ExtendedHash>,      // 0x40
    pub unk50: TablePointer<Vector4>,              // 0x50

    pub unk60: [u32; 4], // 0x60

    pub constant_buffer_slot: u32, // 0x70
    pub constant_buffer: TagHash,  // 0x74

    pub unk78: [u32; 6],
}

#[derive(BinRead, Debug, Clone)]
pub struct SMaterialTextureAssignment {
    /// Material slot to assign to
    pub index: u32,
    _pad: u32,
    pub texture: ExtendedHash,
}

pub struct Technique {
    pub mat: STechnique,
    tag: TagHash,

    pub stage_vertex: TechniqueStage,
    // pub shader_unk1: STechniqueShader,
    // pub shader_unk2: STechniqueShader,
    // pub shader_unk3: STechniqueShader,
    pub stage_pixel: TechniqueStage,
}

impl Technique {
    pub fn all_stages(&self) -> [&TechniqueStage; 2] {
        [&self.stage_pixel, &self.stage_vertex]
    }
}

impl Technique {
    // TODO(cohae): load_shaders is a hack, probably best to use channels so we can remove the dependency on RenderData
    pub fn load(renderer: &Renderer, mat: STechnique, tag: TagHash, load_shaders: bool) -> Self {
        let _span = debug_span!("Load material", hash = %tag).entered();
        Self {
            stage_pixel: TechniqueStage::load(
                renderer,
                &mat.shader_pixel,
                TfxShaderStage::Pixel,
                load_shaders,
            ),
            stage_vertex: TechniqueStage::load(
                renderer,
                &mat.shader_vertex,
                TfxShaderStage::Vertex,
                load_shaders,
            ),
            mat,
            tag,
        }
    }

    pub fn bind(
        &self,
        dcs: &DeviceContextSwapchain,
        render_data: &RenderData,
        stages: ShaderStages,
    ) -> anyhow::Result<()> {
        if stages.contains(ShaderStages::VERTEX) {
            self.stage_vertex.bind(dcs, render_data);
        }

        if stages.contains(ShaderStages::PIXEL) {
            self.stage_pixel.bind(dcs, render_data);
        }

        Ok(())
    }

    pub fn evaluate_bytecode(&self, renderer: &Renderer, render_data: &RenderData) {
        self.stage_pixel
            .evaluate_bytecode(renderer, render_data, self.tag);
        self.stage_vertex
            .evaluate_bytecode(renderer, render_data, self.tag);
    }

    pub fn unbind_textures(&self, dcs: &DeviceContextSwapchain) {
        for s in self.all_stages() {
            s.unbind_textures(dcs);
        }
    }
}

impl Deref for Technique {
    type Target = STechnique;

    fn deref(&self) -> &Self::Target {
        &self.mat
    }
}

pub struct TechniqueStage {
    pub shader: STechniqueShader,
    pub stage: TfxShaderStage,

    cbuffer: Option<ConstantBufferCached<Vec4>>,
    bytecode: RwLock<Option<TfxBytecodeInterpreter>>,
}

impl TechniqueStage {
    pub fn load(
        renderer: &Renderer,
        shader: &STechniqueShader,
        stage: TfxShaderStage,
        load_shaders: bool,
    ) -> Self {
        let cbuffer = if shader.constant_buffer.is_some() {
            let buffer_header_ref = package_manager()
                .get_entry(shader.constant_buffer)
                .unwrap()
                .reference;

            let data_raw = package_manager().read_tag(buffer_header_ref).unwrap();

            let data = bytemuck::cast_slice(&data_raw);
            trace!(
                "Read {} elements cbuffer from {buffer_header_ref:?}",
                data.len()
            );
            let buf = ConstantBufferCached::create_array_init(renderer.dcs.clone(), data).unwrap();

            Some(buf)
        } else if !shader.unk50.is_empty() {
            trace!(
                "Loading float4 cbuffer with {} elements",
                shader.unk50.len()
            );
            let buf = ConstantBufferCached::create_array_init(
                renderer.dcs.clone(),
                bytemuck::cast_slice(&shader.unk50),
            )
            .unwrap();

            Some(buf)
        } else {
            None
        };

        if load_shaders {
            match stage {
                TfxShaderStage::Pixel => {
                    renderer
                        .render_data
                        .load_pshader(&renderer.dcs, shader.shader);
                }
                TfxShaderStage::Vertex => {
                    renderer
                        .render_data
                        .load_vshader(&renderer.dcs, shader.shader);
                }
                TfxShaderStage::Geometry => todo!(),
                TfxShaderStage::Hull => todo!(),
                TfxShaderStage::Compute => todo!(),
                TfxShaderStage::Domain => todo!(),
            }
        }

        let bytecode = match TfxBytecodeOp::parse_all(&shader.bytecode, binrw::Endian::Little) {
            Ok(opcodes) => Some(TfxBytecodeInterpreter::new(opcodes)),
            Err(e) => {
                debug!(
                    "Failed to parse VS TFX bytecode: {e} (data={})",
                    hex::encode(shader.bytecode.data())
                );
                None
            }
        };

        Self {
            shader: shader.clone(),
            stage,
            cbuffer,
            bytecode: RwLock::new(bytecode),
        }
    }

    pub fn bind(&self, dcs: &DeviceContextSwapchain, render_data: &RenderData) {
        unsafe {
            for (si, s) in self.shader.samplers.iter().enumerate() {
                self.stage.set_samplers(
                    dcs,
                    1 + si as u32,
                    Some(&[render_data.samplers.get(&s.key()).cloned()]),
                );
            }

            if let Some(ref cbuffer) = self.cbuffer {
                cbuffer.bind(0, self.stage);
            } else {
                self.stage.set_constant_buffers(dcs, 0, Some(&[None]));
            }

            match self.stage {
                TfxShaderStage::Pixel => {
                    if let Some((ps, _)) = render_data.pshaders.get(&self.shader.shader) {
                        dcs.context().PSSetShader(ps, None);
                    }
                }
                TfxShaderStage::Vertex => {
                    if let Some((vs, _, _)) = render_data.vshaders.get(&self.shader.shader) {
                        dcs.context().VSSetShader(vs, None);
                    }
                }
                TfxShaderStage::Geometry => todo!(),
                TfxShaderStage::Hull => todo!(),
                TfxShaderStage::Compute => todo!(),
                TfxShaderStage::Domain => todo!(),
            }

            for p in &self.shader.textures {
                let tex = render_data
                    .textures
                    .get(&p.texture.key())
                    .unwrap_or(&render_data.fallback_texture);

                self.stage
                    .set_shader_resources(dcs, p.index, Some(&[Some(tex.view.clone())]));
            }
        }
    }

    pub fn evaluate_bytecode(
        &self,
        renderer: &Renderer,
        render_data: &RenderData,
        parent: TagHash,
    ) {
        if let Some(ref cbuffer) = self.cbuffer {
            let _span = info_span!("Evaluating TFX bytecode (VS)").entered();
            let res = if let Some(interpreter) = self.bytecode.read().as_ref() {
                interpreter.evaluate(
                    renderer,
                    render_data,
                    cbuffer,
                    if self.shader.bytecode_constants.is_empty() {
                        &[]
                    } else {
                        bytemuck::cast_slice(&self.shader.bytecode_constants)
                    },
                )
            } else {
                Ok(())
            };

            if !self
                .bytecode
                .read()
                .as_ref()
                .map(|v| v.error_shown)
                .unwrap_or(true)
            {
                if let Err(e) = res {
                    error!(
                        "TFX bytecode evaluation failed for {} ({:?}): {e}",
                        parent, self.stage
                    );
                    self.bytecode.read().as_ref().unwrap().dump(
                        if self.shader.bytecode_constants.is_empty() {
                            &[]
                        } else {
                            bytemuck::cast_slice(&self.shader.bytecode_constants)
                        },
                        cbuffer,
                    );
                    self.bytecode.write().as_mut().unwrap().error_shown = true;
                }
            }
        }
    }

    pub fn unbind_textures(&self, dcs: &DeviceContextSwapchain) {
        for p in &self.shader.textures {
            self.stage.set_shader_resources(dcs, p.index, Some(&[None]));
        }
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

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806da1 {
    pub file_size: u64,
    pub unk8: u64,
    pub unk10: [u32; 8],

    pub bytecode: TablePointer<u8>,
    pub bytecode_constants: TablePointer<Vector4>,
    pub unk50: [u32; 4],
    pub unk60: TablePointer<Vector4>,
}
