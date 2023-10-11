use std::io::SeekFrom;
use std::ops::Deref;

use crate::packages::package_manager;
use crate::render::bytecode::interpreter::TfxBytecodeInterpreter;
use crate::render::bytecode::opcodes::TfxBytecodeOp;
use crate::render::drawcall::ShaderStages;
use crate::render::renderer::Renderer;
use crate::render::{ConstantBuffer, DeviceContextSwapchain, RenderData};
use crate::structure::ExtendedHash;
use crate::structure::{RelPointer, TablePointer};
use crate::types::Vector4;
use binrw::{BinRead, NullString};
use destiny_pkg::TagHash;
use glam::Vec4;

#[derive(BinRead, Debug, Clone)]
pub struct Unk808071e8 {
    pub file_size: u64,
    /// 1 = normal
    /// 2 = depth prepass?
    pub unk8: u32,
    pub unkc: u32,
    pub unk10: u32,
    pub unk14: u32,
    pub unk18: u32,
    pub unk1c: u32,
    pub unk20: u16,
    pub unk22: u16,
    pub unk24: u32,
    pub unk28: [u32; 8],

    #[br(seek_before(SeekFrom::Start(0x70)))]
    pub vertex_shader: TagHash,
    pub unk5c: u32,
    pub vs_textures: TablePointer<Unk80807211>,
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
    pub ps_textures: TablePointer<Unk80807211>,
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
pub struct Unk80807211 {
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

pub struct Material {
    pub mat: Unk808071e8,
    tag: TagHash,

    pub cb0_vs: Option<ConstantBuffer<Vec4>>,
    tfx_bytecode_vs: Option<TfxBytecodeInterpreter>,
    pub cb0_ps: Option<ConstantBuffer<Vec4>>,
    tfx_bytecode_ps: Option<TfxBytecodeInterpreter>,
}

impl Material {
    // TODO(cohae): load_shaders is a hack, i fucking hate locks
    pub fn load(renderer: &Renderer, mat: Unk808071e8, tag: TagHash, load_shaders: bool) -> Self {
        let _span = debug_span!("Load material", hash = %tag).entered();
        let cb0_vs = if mat.unke4.is_some() {
            let buffer_header_ref = package_manager().get_entry(mat.unke4).unwrap().reference;

            let data_raw = package_manager().read_tag(buffer_header_ref).unwrap();
            let data = bytemuck::cast_slice(&data_raw);

            trace!(
                "Read {} elements cbuffer from {buffer_header_ref:?}",
                data.len()
            );
            let buf = ConstantBuffer::create_array_init(renderer.dcs.clone(), data).unwrap();

            Some(buf)
        } else if mat.unka8.len() > 1
            && mat
                .unka8
                .iter()
                .any(|v| v.x != 0.0 || v.y != 0.0 || v.z != 0.0 || v.w != 0.0)
        {
            trace!("Loading float4 cbuffer with {} elements", mat.unk2f8.len());
            let buf = ConstantBuffer::create_array_init(
                renderer.dcs.clone(),
                bytemuck::cast_slice(&mat.unka8),
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

        let cb0_ps = if mat.unk334.is_some() {
            let buffer_header_ref = package_manager().get_entry(mat.unk334).unwrap().reference;

            let data_raw = package_manager().read_tag(buffer_header_ref).unwrap();

            let data = bytemuck::cast_slice(&data_raw);
            trace!(
                "Read {} elements cbuffer from {buffer_header_ref:?}",
                data.len()
            );
            let buf = ConstantBuffer::create_array_init(renderer.dcs.clone(), data).unwrap();

            Some(buf)
        } else if !mat.unk2f8.is_empty()
            && mat
                .unk2f8
                .iter()
                .any(|v| v.x != 0.0 || v.y != 0.0 || v.z != 0.0 || v.w != 0.0)
        {
            trace!("Loading float4 cbuffer with {} elements", mat.unk2f8.len());
            let buf = ConstantBuffer::create_array_init(
                renderer.dcs.clone(),
                bytemuck::cast_slice(&mat.unk2f8),
            )
            .unwrap();

            Some(buf)
        } else {
            None
        };

        if load_shaders {
            renderer
                .render_data
                .load_vshader(&renderer.dcs, mat.vertex_shader);
            renderer
                .render_data
                .load_pshader(&renderer.dcs, mat.pixel_shader);
        }

        let tfx_bytecode_vs =
            match TfxBytecodeOp::parse_all(&mat.vs_bytecode, binrw::Endian::Little) {
                Ok(opcodes) => Some(TfxBytecodeInterpreter::new(opcodes)),
                Err(e) => {
                    debug!(
                        "Failed to parse VS TFX bytecode: {e} (data={})",
                        hex::encode(mat.vs_bytecode.data())
                    );
                    None
                }
            };

        let tfx_bytecode_ps =
            match TfxBytecodeOp::parse_all(&mat.ps_bytecode, binrw::Endian::Little) {
                Ok(opcodes) => Some(TfxBytecodeInterpreter::new(opcodes)),
                Err(e) => {
                    debug!(
                        "Failed to parse PS TFX bytecode: {e} (data={})",
                        hex::encode(mat.ps_bytecode.data())
                    );
                    None
                }
            };

        Self {
            mat,
            tag,
            cb0_vs,
            tfx_bytecode_vs,
            cb0_ps,
            tfx_bytecode_ps,
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
                for (si, s) in self.vs_samplers.iter().enumerate() {
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

                if let Some((vs, _, _)) = render_data.vshaders.get(&self.vertex_shader) {
                    dcs.context().VSSetShader(vs, None);
                } else {
                    // TODO: should still be handled, but not here
                    // anyhow::bail!("No vertex shader/input layout bound");
                }

                for p in &self.vs_textures {
                    let tex = render_data
                        .textures
                        .get(&p.texture.key())
                        .unwrap_or(&render_data.fallback_texture);

                    dcs.context()
                        .VSSetShaderResources(p.index, Some(&[Some(tex.view.clone())]));
                }
            }

            if stages.contains(ShaderStages::PIXEL) {
                for (si, s) in self.ps_samplers.iter().enumerate() {
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
                if let Some((ps, _)) = render_data.pshaders.get(&self.pixel_shader) {
                    dcs.context().PSSetShader(ps, None);
                } else {
                    // TODO: should still be handled, but not here
                    // anyhow::bail!("No pixel shader bound");
                }
                for p in &self.ps_textures {
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

    pub fn evaluate_bytecode(&mut self, renderer: &Renderer) {
        if let Some(ref cb0_vs) = self.cb0_vs {
            if self.tfx_bytecode_vs.is_some() {
                let _span = info_span!("Evaluating TFX bytecode (VS)").entered();
                let res = self.tfx_bytecode_vs.as_mut().unwrap().evaluate(
                    renderer,
                    cb0_vs,
                    if self.mat.vs_bytecode_constants.is_empty() {
                        &[]
                    } else {
                        bytemuck::cast_slice(&self.mat.vs_bytecode_constants)
                    },
                );
                if let Err(e) = res {
                    error!(
                        "TFX bytecode evaluation failed for {} (VS), disabling: {e}",
                        self.tag
                    );
                    self.tfx_bytecode_vs.as_ref().unwrap().dump(
                        if self.mat.vs_bytecode_constants.is_empty() {
                            &[]
                        } else {
                            bytemuck::cast_slice(&self.mat.vs_bytecode_constants)
                        },
                        cb0_vs,
                    );
                    self.tfx_bytecode_vs = None;
                }
            }
        }

        if let Some(ref cb0_ps) = self.cb0_ps {
            if self.tfx_bytecode_ps.is_some() {
                let _span = info_span!("Evaluating TFX bytecode (PS)").entered();
                let res = self.tfx_bytecode_ps.as_mut().unwrap().evaluate(
                    renderer,
                    cb0_ps,
                    if self.mat.ps_bytecode_constants.is_empty() {
                        &[]
                    } else {
                        bytemuck::cast_slice(&self.mat.ps_bytecode_constants)
                    },
                );
                if let Err(e) = res {
                    error!(
                        "TFX bytecode evaluation failed for {} (PS), disabling: {e}",
                        self.tag
                    );
                    self.tfx_bytecode_ps.as_ref().unwrap().dump(
                        if self.mat.ps_bytecode_constants.is_empty() {
                            &[]
                        } else {
                            bytemuck::cast_slice(&self.mat.ps_bytecode_constants)
                        },
                        cb0_ps,
                    );
                    self.tfx_bytecode_ps = None;
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

impl Deref for Material {
    type Target = Unk808071e8;

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
