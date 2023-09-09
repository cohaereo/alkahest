use std::ops::Deref;

use crate::render::{DeviceContextSwapchain, RenderData};
use crate::structure::{RelPointer, TablePointer};
use crate::types::Vector4;
use binrw::{BinRead, NullString};
use destiny_pkg::TagHash;

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
    pub unk68: TablePointer<u8>,
    pub unk78: TablePointer<Vector4>,
    pub vs_samplers: TablePointer<Unk808073f3>,
    pub unk98: TablePointer<Vector4>,
    pub unka8: [u32; 9],

    pub unkcc: TagHash,
    pub unkd0: [u32; 126],

    pub pixel_shader: TagHash,
    pub unk2cc: u32,
    pub ps_textures: TablePointer<Unk80807211>,
    pub unk2e0: u64,
    pub unk2e8: TablePointer<u8>,
    pub unk2f8: TablePointer<Vector4>,
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

pub struct Material(pub Unk808071e8, pub TagHash);

impl Material {
    pub fn tag(&self) -> TagHash {
        self.1
    }

    pub fn bind(
        &self,
        dcs: &DeviceContextSwapchain,
        render_data: &RenderData,
    ) -> anyhow::Result<()> {
        unsafe {
            for (si, s) in self.vs_samplers.iter().enumerate() {
                dcs.context().VSSetSamplers(
                    1 + si as u32,
                    Some(&[render_data.samplers.get(&s.sampler.0).cloned()]),
                );
            }
            for (si, s) in self.ps_samplers.iter().enumerate() {
                dcs.context().PSSetSamplers(
                    1 + si as u32,
                    Some(&[render_data.samplers.get(&s.sampler.0).cloned()]),
                );
            }

            if let Some(cbuffer) = render_data.cbuffers_ps.get(&self.tag().into()) {
                dcs.context()
                    .PSSetConstantBuffers(0, Some(&[Some(cbuffer.buffer().clone())]));
            } else {
                dcs.context().PSSetConstantBuffers(0, Some(&[None]));
            }

            if let Some(cbuffer) = render_data.cbuffers_vs.get(&self.tag().into()) {
                dcs.context()
                    .VSSetConstantBuffers(0, Some(&[Some(cbuffer.buffer().clone())]));
            } else {
                dcs.context().VSSetConstantBuffers(0, Some(&[None]));
            }

            if let Some((vs, Some(input_layout))) = render_data.vshaders.get(&self.vertex_shader.0)
            {
                dcs.context().IASetInputLayout(input_layout);
                dcs.context().VSSetShader(vs, None);
            } else {
                // TODO: should still be handled, but not here
                // anyhow::bail!("No vertex shader/input layout bound");
            }

            if let Some((ps, _)) = render_data.pshaders.get(&self.pixel_shader.0) {
                dcs.context().PSSetShader(ps, None);
            } else {
                // TODO: should still be handled, but not here
                // anyhow::bail!("No pixel shader bound");
            }

            for p in &self.vs_textures {
                // TODO(cohae): Bind error texture on error
                if let Some(t) = render_data.textures.get(&p.texture.0) {
                    dcs.context()
                        .VSSetShaderResources(p.index, Some(&[Some(t.view.clone())]));
                }
            }

            for p in &self.ps_textures {
                // TODO(cohae): Bind error texture on error
                if let Some(t) = render_data.textures.get(&p.texture.0) {
                    dcs.context()
                        .PSSetShaderResources(p.index, Some(&[Some(t.view.clone())]));
                }
            }
        }

        Ok(())
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
        &self.0
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
