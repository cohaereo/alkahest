use std::ops::Deref;

use alkahest_data::{
    technique::{STechnique, STechniqueShader},
    tfx::TfxShaderStage,
};
use alkahest_pm::package_manager;
use anyhow::{ensure, Context};
use destiny_pkg::TagHash;
use glam::Vec4;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11ComputeShader, ID3D11DomainShader, ID3D11GeometryShader, ID3D11HullShader,
    ID3D11PixelShader, ID3D11SamplerState, ID3D11VertexShader,
};

use crate::{
    gpu::{buffer::ConstantBufferCached, texture::Texture, GpuContext},
    handle::Handle,
    renderer::Renderer,
    tfx::bytecode::interpreter::TfxBytecodeInterpreter,
    util::d3d::D3dResource,
};

pub struct Technique {
    pub tech: STechnique,
    pub hash: TagHash,

    pub stage_vertex: Option<TechniqueStage>,
    // pub stage_hull: Option<TechniqueStage>,
    // pub stage_domain: Option<TechniqueStage>,
    pub stage_geometry: Option<TechniqueStage>,
    pub stage_pixel: Option<TechniqueStage>,
    pub stage_compute: Option<TechniqueStage>,
}

impl Technique {
    pub fn all_stages(&self) -> [(&STechniqueShader, Option<&TechniqueStage>); 4] {
        [
            (&self.tech.shader_pixel, self.stage_pixel.as_ref()),
            (&self.tech.shader_geometry, self.stage_geometry.as_ref()),
            (&self.tech.shader_vertex, self.stage_vertex.as_ref()),
            (&self.tech.shader_compute, self.stage_compute.as_ref()),
        ]
    }
    pub fn all_stages_mut(&mut self) -> [(&STechniqueShader, Option<&mut TechniqueStage>); 4] {
        [
            (&self.tech.shader_pixel, self.stage_pixel.as_mut()),
            (&self.tech.shader_geometry, self.stage_geometry.as_mut()),
            (&self.tech.shader_vertex, self.stage_vertex.as_mut()),
            (&self.tech.shader_compute, self.stage_compute.as_mut()),
        ]
    }
}

impl Technique {
    pub fn bind(&self, renderer: &Renderer) -> anyhow::Result<()> {
        let states = renderer.gpu.current_states.load().select(&self.tech.states);
        if let Some(u) = states.blend_state() {
            renderer.gpu.set_blend_state(u);
        }
        if let Some(u) = states.depth_stencil_state() {
            renderer.gpu.set_depth_stencil_state(u);
        }
        if let Some(u) = states.rasterizer_state() {
            renderer.gpu.set_rasterizer_state(u);
        }
        if let Some(u) = states.depth_bias_state() {
            renderer.gpu.set_depth_bias(u);
        }

        let ctx = renderer.gpu.context();
        unsafe {
            match self.unk8 {
                1 => {
                    self.stage_vertex
                        .as_ref()
                        .context("Vertex stage not set")?
                        .bind(renderer)?;
                    self.stage_pixel
                        .as_ref()
                        .context("Pixel stage not set")?
                        .bind(renderer)?;

                    ctx.GSSetShader(None, None);
                    ctx.HSSetShader(None, None);
                    ctx.DSSetShader(None, None);
                    ctx.CSSetShader(None, None);
                }
                2 => {
                    self.stage_vertex
                        .as_ref()
                        .context("Vertex stage not set")?
                        .bind(renderer)?;

                    ctx.PSSetShader(None, None);
                    ctx.GSSetShader(None, None);
                    ctx.HSSetShader(None, None);
                    ctx.DSSetShader(None, None);
                    ctx.CSSetShader(None, None);
                }
                3 => {
                    self.stage_vertex
                        .as_ref()
                        .context("Vertex stage not set")?
                        .bind(renderer)?;
                    self.stage_geometry
                        .as_ref()
                        .context("Geometry stage not set")?
                        .bind(renderer)?;

                    ctx.GSSetShader(None, None);
                    ctx.HSSetShader(None, None);
                    ctx.DSSetShader(None, None);
                    ctx.CSSetShader(None, None);
                }
                4 => {
                    anyhow::bail!(
                        "Unsupported shader stage HS+DS for shader bind type: {}",
                        self.unk8
                    );
                }
                5 => {
                    anyhow::bail!(
                        "Unsupported shader stage HS+DS for shader bind type: {}",
                        self.unk8
                    );
                }
                6 => {
                    self.stage_compute
                        .as_ref()
                        .context("Compute stage not set")?
                        .bind(renderer)?;
                }
                // Seems to be primarily used by postprocessing shaders
                0 => {
                    self.stage_vertex
                        .as_ref()
                        .context("Vertex stage not set")?
                        .bind(renderer)?;
                    self.stage_pixel
                        .as_ref()
                        .context("Pixel stage not set")?
                        .bind(renderer)?;
                    self.stage_compute
                        .as_ref()
                        .context("Pixel stage not set")?
                        .bind(renderer)?;

                    ctx.GSSetShader(None, None);
                    ctx.HSSetShader(None, None);
                    ctx.DSSetShader(None, None);
                }
                u => {
                    anyhow::bail!("Unsupported shader bind type: {u}")
                }
            }
        }

        Ok(())
    }
}

impl Deref for Technique {
    type Target = STechnique;

    fn deref(&self) -> &Self::Target {
        &self.tech
    }
}

pub struct TechniqueStage {
    pub shader: STechniqueShader,
    pub stage: TfxShaderStage,

    // cohae: Due to the way the asset system works, these are loaded in the asset manager itself
    // instead of in the same task as the technique loaderin order to avoid loading textures multiple times
    pub textures: Vec<(u32, Handle<Texture>)>,
    pub samplers: Vec<Option<ID3D11SamplerState>>,
    pub shader_module: ShaderModule,

    pub cbuffer: Option<ConstantBufferCached<Vec4>>,
    pub bytecode: Option<TfxBytecodeInterpreter>,
}

impl TechniqueStage {
    pub fn bind(&self, renderer: &Renderer) -> anyhow::Result<()> {
        self.shader_module.bind(&renderer.gpu);
        for (slot, tex) in &self.textures {
            if let Some(tex) = renderer.data.lock().asset_manager.textures.get_shared(tex) {
                tex.bind(&renderer.gpu, *slot, self.stage);
            } else {
                renderer
                    .gpu
                    .fallback_texture
                    .bind(&renderer.gpu, *slot, self.stage);
            }
        }

        if let (Some(cbuffer), Some(bytecode)) = (&self.cbuffer, &self.bytecode) {
            bytecode.evaluate(
                &renderer.gpu,
                &renderer.data.lock().externs,
                cbuffer,
                &self.shader.constants.bytecode_constants,
                &self.samplers,
            )?;
        }

        if self.shader.constants.constant_buffer_slot != -1 {
            if let Some(cbuffer) = &self.cbuffer {
                renderer.gpu.bind_cbuffer(
                    self.shader.constants.constant_buffer_slot as u32,
                    Some(cbuffer.buffer().clone()),
                    self.stage,
                );
            }
        }

        Ok(())
    }
}

pub enum ShaderModule {
    Vertex(ID3D11VertexShader),
    Pixel(ID3D11PixelShader),
    Geometry(ID3D11GeometryShader),
    Hull(ID3D11HullShader),
    Domain(ID3D11DomainShader),
    Compute(ID3D11ComputeShader),
}

impl ShaderModule {
    pub fn bind(&self, gctx: &GpuContext) {
        unsafe {
            match self {
                ShaderModule::Vertex(shader) => gctx.context().VSSetShader(shader, None),
                ShaderModule::Pixel(shader) => gctx.bind_pixel_shader(shader),
                ShaderModule::Geometry(shader) => gctx.context().GSSetShader(shader, None),
                ShaderModule::Hull(shader) => gctx.context().HSSetShader(shader, None),
                ShaderModule::Domain(shader) => gctx.context().DSSetShader(shader, None),
                ShaderModule::Compute(shader) => gctx.context().CSSetShader(shader, None),
            }
        }
    }

    pub fn with_name(self, name: &str) -> Self {
        self.set_name(name);
        self
    }

    pub fn set_name(&self, name: &str) {
        match self {
            ShaderModule::Vertex(shader) => shader.set_debug_name(name),
            ShaderModule::Pixel(shader) => shader.set_debug_name(name),
            ShaderModule::Geometry(shader) => shader.set_debug_name(name),
            ShaderModule::Hull(shader) => shader.set_debug_name(name),
            ShaderModule::Domain(shader) => shader.set_debug_name(name),
            ShaderModule::Compute(shader) => shader.set_debug_name(name),
        }
    }

    pub fn load(gctx: &GpuContext, hash: TagHash) -> anyhow::Result<Self> {
        let entry = package_manager()
            .get_entry(hash)
            .context("Entry not found")?;
        ensure!(
            entry.file_type == 33 && entry.file_subtype <= 6,
            "Shader header type mismatch"
        );

        let data = package_manager()
            .read_tag(entry.reference)
            .context("Failed to read shader data")?;

        match entry.file_subtype {
            0 => {
                let mut shader = None;
                unsafe {
                    gctx.device
                        .CreatePixelShader(&data, None, Some(&mut shader))?;
                }
                Ok(ShaderModule::Pixel(shader.unwrap()))
            }
            1 => {
                let mut shader = None;
                unsafe {
                    gctx.device
                        .CreateVertexShader(&data, None, Some(&mut shader))?;
                }
                Ok(ShaderModule::Vertex(shader.unwrap()))
            }
            2 => {
                let mut shader = None;
                unsafe {
                    gctx.device
                        .CreateGeometryShader(&data, None, Some(&mut shader))?;
                }
                Ok(ShaderModule::Geometry(shader.unwrap()))
            }
            3..=5 => {
                anyhow::bail!("Unsupported shader type: {}", entry.file_subtype);
            }
            6 => {
                let mut shader = None;
                unsafe {
                    gctx.device
                        .CreateComputeShader(&data, None, Some(&mut shader))?;
                }
                Ok(ShaderModule::Compute(shader.unwrap()))
            }
            _ => unreachable!(),
        }
    }
}
