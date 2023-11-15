use std::ops::Deref;

use crate::packages::package_manager;
use crate::render::bytecode::externs::TfxShaderStage;
use crate::render::bytecode::interpreter::TfxBytecodeInterpreter;
use crate::render::bytecode::opcodes::TfxBytecodeOp;
use crate::render::drawcall::ShaderStages;
use crate::render::renderer::Renderer;
use crate::render::{ConstantBuffer, DeviceContextSwapchain, RenderData};
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

    pub cb0_vs: Option<ConstantBuffer<Vec4>>,
    tfx_bytecode_vs: RwLock<Option<TfxBytecodeInterpreter>>,
    pub cb0_ps: Option<ConstantBuffer<Vec4>>,
    tfx_bytecode_ps: RwLock<Option<TfxBytecodeInterpreter>>,
}

impl Technique {
    // TODO(cohae): load_shaders is a hack, i fucking hate locks
    pub fn load(renderer: &Renderer, mat: STechnique, tag: TagHash, load_shaders: bool) -> Self {
        let _span = debug_span!("Load material", hash = %tag).entered();
        let cb0_vs = if mat.shader_vertex.constant_buffer.is_some() {
            let buffer_header_ref = package_manager()
                .get_entry(mat.shader_vertex.constant_buffer)
                .unwrap()
                .reference;

            let data_raw = package_manager().read_tag(buffer_header_ref).unwrap();
            let data = bytemuck::cast_slice(&data_raw);

            trace!(
                "Read {} elements cbuffer from {buffer_header_ref:?}",
                data.len()
            );
            let buf = ConstantBuffer::create_array_init(renderer.dcs.clone(), data).unwrap();

            Some(buf)
        } else if mat.shader_vertex.unk50.len() > 1
            && mat
                .shader_vertex
                .unk50
                .iter()
                .any(|v| v.x != 0.0 || v.y != 0.0 || v.z != 0.0 || v.w != 0.0)
        {
            trace!(
                "Loading float4 cbuffer with {} elements",
                mat.shader_vertex.unk50.len()
            );
            let buf = ConstantBuffer::create_array_init(
                renderer.dcs.clone(),
                bytemuck::cast_slice(&mat.shader_vertex.unk50),
            )
            .unwrap();

            Some(buf)
        } else {
            trace!("Loading default float4 cbuffer");
            let buf = ConstantBuffer::create_array_init(
                renderer.dcs.clone(),
                &[Vec4::new(1.0, 1.0, 1.0, 1.0)],
            )
            .unwrap();

            Some(buf)
        };

        let cb0_ps = if mat.shader_pixel.constant_buffer.is_some() {
            let buffer_header_ref = package_manager()
                .get_entry(mat.shader_pixel.constant_buffer)
                .unwrap()
                .reference;

            let data_raw = package_manager().read_tag(buffer_header_ref).unwrap();

            let data = bytemuck::cast_slice(&data_raw);
            trace!(
                "Read {} elements cbuffer from {buffer_header_ref:?}",
                data.len()
            );
            let buf = ConstantBuffer::create_array_init(renderer.dcs.clone(), data).unwrap();

            Some(buf)
        } else if !mat.shader_pixel.unk50.is_empty()
            && mat
                .shader_pixel
                .unk50
                .iter()
                .any(|v| v.x != 0.0 || v.y != 0.0 || v.z != 0.0 || v.w != 0.0)
        {
            trace!(
                "Loading float4 cbuffer with {} elements",
                mat.shader_pixel.unk50.len()
            );
            let buf = ConstantBuffer::create_array_init(
                renderer.dcs.clone(),
                bytemuck::cast_slice(&mat.shader_pixel.unk50),
            )
            .unwrap();

            Some(buf)
        } else {
            None
        };

        if load_shaders {
            renderer
                .render_data
                .load_vshader(&renderer.dcs, mat.shader_vertex.shader);
            renderer
                .render_data
                .load_pshader(&renderer.dcs, mat.shader_pixel.shader);
        }

        let tfx_bytecode_vs =
            match TfxBytecodeOp::parse_all(&mat.shader_vertex.bytecode, binrw::Endian::Little) {
                Ok(opcodes) => Some(TfxBytecodeInterpreter::new(opcodes)),
                Err(e) => {
                    debug!(
                        "Failed to parse VS TFX bytecode: {e} (data={})",
                        hex::encode(mat.shader_vertex.bytecode.data())
                    );
                    None
                }
            };

        let tfx_bytecode_ps =
            match TfxBytecodeOp::parse_all(&mat.shader_pixel.bytecode, binrw::Endian::Little) {
                Ok(opcodes) => Some(TfxBytecodeInterpreter::new(opcodes)),
                Err(e) => {
                    debug!(
                        "Failed to parse PS TFX bytecode: {e} (data={})",
                        hex::encode(mat.shader_pixel.bytecode.data())
                    );
                    None
                }
            };

        Self {
            mat,
            tag,
            cb0_vs,
            cb0_ps,
            tfx_bytecode_vs: RwLock::new(tfx_bytecode_vs),
            tfx_bytecode_ps: RwLock::new(tfx_bytecode_ps),
        }
    }

    // pub fn tag(&self) -> TagHash {
    //     self.tag
    // }

    pub fn bind(
        &self,
        dcs: &DeviceContextSwapchain,
        render_data: &RenderData,
        stages: ShaderStages,
    ) -> anyhow::Result<()> {
        unsafe {
            if stages.contains(ShaderStages::VERTEX) {
                for (si, s) in self.shader_vertex.samplers.iter().enumerate() {
                    dcs.context().VSSetSamplers(
                        1 + si as u32,
                        Some(&[render_data.samplers.get(&s.key()).cloned()]),
                    );
                }

                if let Some(ref cbuffer) = self.cb0_vs {
                    dcs.context()
                        .VSSetConstantBuffers(0, Some(&[Some(cbuffer.buffer().clone())]));
                } else {
                    dcs.context().VSSetConstantBuffers(0, Some(&[None]));
                }

                if let Some((vs, _, _)) = render_data.vshaders.get(&self.shader_vertex.shader) {
                    dcs.context().VSSetShader(vs, None);
                } else {
                    // TODO: should still be handled, but not here
                    // anyhow::bail!("No vertex shader/input layout bound");
                }

                for p in &self.shader_vertex.textures {
                    let tex = render_data
                        .textures
                        .get(&p.texture.key())
                        .unwrap_or(&render_data.fallback_texture);

                    dcs.context()
                        .VSSetShaderResources(p.index, Some(&[Some(tex.view.clone())]));
                }
            }

            if stages.contains(ShaderStages::PIXEL) {
                for (si, s) in self.shader_pixel.samplers.iter().enumerate() {
                    dcs.context().PSSetSamplers(
                        1 + si as u32,
                        Some(&[render_data.samplers.get(&s.key()).cloned()]),
                    );
                }

                if let Some(ref cbuffer) = self.cb0_ps {
                    dcs.context()
                        .PSSetConstantBuffers(0, Some(&[Some(cbuffer.buffer().clone())]));
                } else {
                    dcs.context().PSSetConstantBuffers(0, Some(&[None]));
                }
                if let Some((ps, _)) = render_data.pshaders.get(&self.shader_pixel.shader) {
                    dcs.context().PSSetShader(ps, None);
                } else {
                    // TODO: should still be handled, but not here
                    // anyhow::bail!("No pixel shader bound");
                }
                for p in &self.shader_pixel.textures {
                    let tex = render_data
                        .textures
                        .get(&p.texture.key())
                        .unwrap_or(&render_data.fallback_texture);

                    dcs.context()
                        .PSSetShaderResources(p.index, Some(&[Some(tex.view.clone())]));
                }
            }
        }

        Ok(())
    }

    pub fn evaluate_bytecode(&self, renderer: &Renderer, render_data: &RenderData) {
        if let Some(ref cb0_vs) = self.cb0_vs {
            let _span = info_span!("Evaluating TFX bytecode (VS)").entered();
            let res = if let Some(interpreter) = self.tfx_bytecode_vs.read().as_ref() {
                interpreter.evaluate(
                    renderer,
                    render_data,
                    cb0_vs,
                    if self.mat.shader_vertex.bytecode_constants.is_empty() {
                        &[]
                    } else {
                        bytemuck::cast_slice(&self.mat.shader_vertex.bytecode_constants)
                    },
                )
            } else {
                Ok(())
            };

            if let Err(e) = res {
                error!(
                    "TFX bytecode evaluation failed for {} (VS), disabling: {e}",
                    self.tag
                );
                self.tfx_bytecode_vs.read().as_ref().unwrap().dump(
                    if self.mat.shader_vertex.bytecode_constants.is_empty() {
                        &[]
                    } else {
                        bytemuck::cast_slice(&self.mat.shader_vertex.bytecode_constants)
                    },
                    cb0_vs,
                );
                *self.tfx_bytecode_vs.write() = None;
            }
        }
        if let Some(ref cb0_ps) = self.cb0_ps {
            let _span = info_span!("Evaluating TFX bytecode (PS)").entered();
            let res = if let Some(interpreter) = self.tfx_bytecode_ps.read().as_ref() {
                interpreter.evaluate(
                    renderer,
                    render_data,
                    cb0_ps,
                    if self.mat.shader_pixel.bytecode_constants.is_empty() {
                        &[]
                    } else {
                        bytemuck::cast_slice(&self.mat.shader_pixel.bytecode_constants)
                    },
                )
            } else {
                Ok(())
            };

            if let Err(e) = res {
                error!(
                    "TFX bytecode evaluation failed for {} (PS), disabling: {e}",
                    self.tag
                );
                self.tfx_bytecode_ps.read().as_ref().unwrap().dump(
                    if self.mat.shader_pixel.bytecode_constants.is_empty() {
                        &[]
                    } else {
                        bytemuck::cast_slice(&self.mat.shader_pixel.bytecode_constants)
                    },
                    cb0_ps,
                );
                *self.tfx_bytecode_ps.write() = None;
            }
        }
    }

    pub fn unbind_textures(&self, dcs: &DeviceContextSwapchain) {
        unsafe {
            for p in &self.shader_vertex.textures {
                dcs.context().VSSetShaderResources(p.index, Some(&[None]));
            }

            for p in &self.shader_pixel.textures {
                dcs.context().PSSetShaderResources(p.index, Some(&[None]));
            }
        }
    }
}

impl Deref for Technique {
    type Target = STechnique;

    fn deref(&self) -> &Self::Target {
        &self.mat
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
