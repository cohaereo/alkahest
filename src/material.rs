use std::ops::Deref;
use std::sync::Arc;

use crate::packages::package_manager;
use crate::render::bytecode::interpreter::TfxBytecodeInterpreter;
use crate::render::bytecode::opcodes::TfxBytecodeOp;
use crate::render::renderer::Renderer;
use crate::render::{ConstantBuffer, DeviceContextSwapchain, RenderData};
use crate::structure::{RelPointer, TablePointer};
use crate::types::Vector4;
use binrw::{BinRead, NullString};
use destiny_pkg::TagHash;
use glam::Vec4;

#[derive(BinRead, Debug, Clone)]
pub struct Unk808071e8 {
    pub file_size: u64,
    /// 1 = ??
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

    pub vertex_shader: TagHash,
    pub unk4c: u32,
    pub vs_textures: TablePointer<Unk80807211>,
    pub unk60: u64,
    pub vs_bytecode: TablePointer<u8>,
    pub vs_bytecode_constants: TablePointer<Vector4>,
    pub vs_samplers: TablePointer<Unk808073f3>,
    pub unk98: TablePointer<Vector4>,
    pub unka8: [u32; 9],

    pub unkcc: TagHash,
    pub unkd0: [u32; 126],

    pub pixel_shader: TagHash,
    pub unk2cc: u32,
    pub ps_textures: TablePointer<Unk80807211>,
    pub unk2e0: u64,
    pub ps_bytecode: TablePointer<u8>,
    pub ps_bytecode_constants: TablePointer<Vector4>,
    pub ps_samplers: TablePointer<Unk808073f3>,
    pub unk318: TablePointer<Vector4>,
    pub unk328: [u32; 9],

    /// Pointer to a float4 buffer, usually passed into cbuffer0
    pub unk34c: TagHash,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80807211 {
    /// Material slot to assign to
    pub index: u32,
    pub texture: TagHash,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk808073f3 {
    pub sampler: TagHash,
    pub unk4: u32,
    pub unk8: u32,
    pub unkc: u32,
}

pub struct Material {
    pub mat: Unk808071e8,
    tag: TagHash,

    pub cb0_vs: Option<ConstantBuffer<Vec4>>,
    tfx_bytecode_vs: Option<TfxBytecodeInterpreter>,
    pub cb0_ps: Option<ConstantBuffer<Vec4>>,
    tfx_bytecode_ps: Option<TfxBytecodeInterpreter>,
}

impl Material {
    pub fn load(dcs: Arc<DeviceContextSwapchain>, mat: Unk808071e8, tag: TagHash) -> Self {
        let _span = debug_span!("Load material", hash = %tag).entered();
        let cb0_vs = if mat.unkcc.is_valid() {
            let buffer_header_ref = package_manager().get_entry(mat.unkcc).unwrap().reference;

            let data_raw = package_manager().read_tag(buffer_header_ref).unwrap();
            let data = bytemuck::cast_slice(&data_raw);

            trace!(
                "Read {} elements cbuffer from {buffer_header_ref:?}",
                data.len()
            );
            let buf = ConstantBuffer::create_array_init(dcs.clone(), data).unwrap();

            Some(buf)
        } else if mat.unk98.len() > 1
            && mat
                .unk98
                .iter()
                .any(|v| v.x != 0.0 || v.y != 0.0 || v.z != 0.0 || v.w != 0.0)
        {
            trace!("Loading float4 cbuffer with {} elements", mat.unk318.len());
            let buf =
                ConstantBuffer::create_array_init(dcs.clone(), bytemuck::cast_slice(&mat.unk98))
                    .unwrap();

            Some(buf)
        } else {
            trace!("Loading default float4 cbuffer");
            let buf =
                ConstantBuffer::create_array_init(dcs.clone(), &[Vec4::new(1.0, 1.0, 1.0, 1.0)])
                    .unwrap();

            Some(buf)
        };

        let cb0_ps = if mat.unk34c.is_valid() {
            let buffer_header_ref = package_manager().get_entry(mat.unk34c).unwrap().reference;

            let data_raw = package_manager().read_tag(buffer_header_ref).unwrap();

            let data = bytemuck::cast_slice(&data_raw);
            trace!(
                "Read {} elements cbuffer from {buffer_header_ref:?}",
                data.len()
            );
            let buf = ConstantBuffer::create_array_init(dcs.clone(), data).unwrap();

            Some(buf)
        } else if !mat.unk318.is_empty()
            && mat
                .unk318
                .iter()
                .any(|v| v.x != 0.0 || v.y != 0.0 || v.z != 0.0 || v.w != 0.0)
        {
            trace!("Loading float4 cbuffer with {} elements", mat.unk318.len());
            let buf =
                ConstantBuffer::create_array_init(dcs.clone(), bytemuck::cast_slice(&mat.unk318))
                    .unwrap();

            Some(buf)
        } else {
            None
        };

        // if tag.0 == u32::from_be(0x5c44eb80) {
        // println!("{}", hex::encode(&mat.ps_bytecode.data()));
        // let e =
        //     TfxBytecodeOp::parse_all(&mat.vs_bytecode, binrw::Endian::Little).unwrap_or_default();
        // if !e.is_empty() {
        //     println!("VS {tag} length={}", e.len());
        //     println!("\t{:#?}", mat.vs_bytecode_constants);
        //     println!("\t{e:#x?}");
        // }
        // let e =
        //     TfxBytecodeOp::parse_all(&mat.ps_bytecode, binrw::Endian::Little).unwrap_or_default();
        // if !e.is_empty() {
        //     println!("PS {tag} length={}", e.len());
        //     println!("\t{:#?}", mat.ps_bytecode_constants);
        //     println!("\t{e:#x?}");
        // }

        // TODO(cohae): error checking
        let tfx_bytecode_vs = TfxBytecodeOp::parse_all(&mat.vs_bytecode, binrw::Endian::Little)
            .ok()
            .map(TfxBytecodeInterpreter::new);

        let tfx_bytecode_ps = TfxBytecodeOp::parse_all(&mat.ps_bytecode, binrw::Endian::Little)
            .ok()
            .map(TfxBytecodeInterpreter::new);

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
    ) -> anyhow::Result<()> {
        unsafe {
            for (si, s) in self.vs_samplers.iter().enumerate() {
                dcs.context().VSSetSamplers(
                    1 + si as u32,
                    Some(&[render_data.samplers.get(&s.sampler).cloned()]),
                );
            }
            for (si, s) in self.ps_samplers.iter().enumerate() {
                dcs.context().PSSetSamplers(
                    1 + si as u32,
                    Some(&[render_data.samplers.get(&s.sampler).cloned()]),
                );
            }

            if let Some(ref cbuffer) = self.cb0_ps {
                dcs.context()
                    .PSSetConstantBuffers(0, Some(&[Some(cbuffer.buffer().clone())]));
            } else {
                dcs.context().PSSetConstantBuffers(0, Some(&[None]));
            }

            if let Some(ref cbuffer) = self.cb0_vs {
                dcs.context()
                    .VSSetConstantBuffers(0, Some(&[Some(cbuffer.buffer().clone())]));
            } else {
                dcs.context().VSSetConstantBuffers(0, Some(&[None]));
            }

            if let Some((vs, Some(input_layout))) = render_data.vshaders.get(&self.vertex_shader) {
                dcs.context().IASetInputLayout(input_layout);
                dcs.context().VSSetShader(vs, None);
            } else {
                // TODO: should still be handled, but not here
                // anyhow::bail!("No vertex shader/input layout bound");
            }

            if let Some((ps, _)) = render_data.pshaders.get(&self.pixel_shader) {
                dcs.context().PSSetShader(ps, None);
            } else {
                // TODO: should still be handled, but not here
                // anyhow::bail!("No pixel shader bound");
            }

            for p in &self.vs_textures {
                // TODO(cohae): Bind error texture on error
                if let Some(t) = render_data.textures.get(&p.texture) {
                    dcs.context()
                        .VSSetShaderResources(p.index, Some(&[Some(t.view.clone())]));
                }
            }

            for p in &self.ps_textures {
                // TODO(cohae): Bind error texture on error
                if let Some(t) = render_data.textures.get(&p.texture) {
                    dcs.context()
                        .PSSetShaderResources(p.index, Some(&[Some(t.view.clone())]));
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
