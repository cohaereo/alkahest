// pub trait Scope {
//     fn size(&self) -> usize;
// }

use std::io::Write;

use alkahest_data::{
    render_globals::{SScope, SScopeStage},
    tfx::TfxShaderStage,
};
use alkahest_pm::package_manager;
use destiny_pkg::TagHash;
use glam::{Mat4, Vec2, Vec3, Vec4};
use tiger_parse::PackageManagerExt;
use windows::Win32::Graphics::Direct3D11::ID3D11SamplerState;

use crate::{
    gpu::{buffer::ConstantBufferCached, GpuContext, SharedGpuContext},
    loaders::AssetManager,
    tfx::{
        bytecode::{interpreter::TfxBytecodeInterpreter, opcodes::TfxBytecodeOp},
        externs::ExternStorage,
        technique::ShaderModule,
    },
};

/// A scope generated through TFX expressions.
pub struct TfxScope {
    scope: SScope,

    stage_pixel: Option<TfxScopeStage>,
    stage_vertex: Option<TfxScopeStage>,
    stage_geometry: Option<TfxScopeStage>,
    stage_compute: Option<TfxScopeStage>,
}

impl TfxScope {
    pub fn load(scope: SScope, gctx: SharedGpuContext) -> anyhow::Result<TfxScope> {
        let sscope = scope.clone();
        let stage_vertex = if sscope.stage_vertex.constant_buffer_slot != -1 {
            TfxScopeStage::load(sscope.stage_vertex, TfxShaderStage::Vertex, gctx.clone())?
        } else {
            None
        };

        let stage_pixel = if sscope.stage_pixel.constant_buffer_slot != -1 {
            TfxScopeStage::load(sscope.stage_pixel, TfxShaderStage::Pixel, gctx.clone())?
        } else {
            None
        };

        let stage_geometry = if sscope.stage_geometry.constant_buffer_slot != -1 {
            TfxScopeStage::load(
                sscope.stage_geometry,
                TfxShaderStage::Geometry,
                gctx.clone(),
            )?
        } else {
            None
        };

        let stage_compute = if sscope.stage_compute.constant_buffer_slot != -1 {
            TfxScopeStage::load(sscope.stage_compute, TfxShaderStage::Compute, gctx.clone())?
        } else {
            None
        };

        Ok(TfxScope {
            scope,
            stage_pixel,
            stage_vertex,
            stage_geometry,
            stage_compute,
        })
    }

    pub fn bind(
        &self,
        gctx: &GpuContext,
        asset_manager: &AssetManager,
        externs: &ExternStorage,
    ) -> anyhow::Result<()> {
        if let Some(stage) = &self.stage_vertex {
            stage.bind(gctx, asset_manager, externs)?;
        }

        if let Some(stage) = &self.stage_pixel {
            stage.bind(gctx, asset_manager, externs)?;
        }

        if let Some(stage) = &self.stage_geometry {
            stage.bind(gctx, asset_manager, externs)?;
        }

        if let Some(stage) = &self.stage_compute {
            stage.bind(gctx, asset_manager, externs)?;
        }

        Ok(())
    }
}

pub struct TfxScopeStage {
    stage: SScopeStage,
    shader_stage: TfxShaderStage,

    samplers: Vec<Option<ID3D11SamplerState>>,

    cbuffer: Option<ConstantBufferCached<Vec4>>,
    bytecode: Option<TfxBytecodeInterpreter>,
}

impl TfxScopeStage {
    pub fn load(
        stage: SScopeStage,
        shader_stage: TfxShaderStage,
        gctx: SharedGpuContext,
    ) -> anyhow::Result<Option<TfxScopeStage>> {
        let cbuffer = if stage.constant_buffer.is_some() {
            let buffer_header_ref = package_manager()
                .get_entry(stage.constant_buffer)
                .unwrap()
                .reference;

            let data_raw = package_manager().read_tag(buffer_header_ref).unwrap();

            let data = bytemuck::cast_slice(&data_raw);
            let buf = ConstantBufferCached::create_array_init(gctx.clone(), data).unwrap();

            Some(buf)
        } else if !stage.unk38.is_empty() {
            let buf = ConstantBufferCached::create_array_init(
                gctx.clone(),
                bytemuck::cast_slice(&stage.unk38),
            )
            .unwrap();

            Some(buf)
        } else {
            None
        };

        let bytecode = match TfxBytecodeOp::parse_all(&stage.bytecode, binrw::Endian::Little) {
            Ok(opcodes) => Some(TfxBytecodeInterpreter::new(opcodes)),
            Err(e) => {
                debug!(
                    "Failed to parse VS TFX bytecode: {e:?} (data={})",
                    hex::encode(&stage.bytecode)
                );
                None
            }
        };

        let mut samplers = vec![];
        for sampler in stage.samplers.iter() {
            samplers.push(crate::loaders::technique::load_sampler(&gctx, sampler.hash32()).ok());
        }

        Ok(Some(TfxScopeStage {
            stage,
            shader_stage,
            samplers,
            cbuffer,
            bytecode,
        }))
    }

    pub fn bind(
        &self,
        gctx: &GpuContext,
        asset_manager: &AssetManager,
        externs: &ExternStorage,
    ) -> anyhow::Result<()> {
        if let (Some(cbuffer), Some(bytecode)) = (&self.cbuffer, &self.bytecode) {
            bytecode.evaluate(
                gctx,
                externs,
                cbuffer,
                asset_manager,
                &self.stage.bytecode_constants,
                &self.samplers,
            )?;
        }

        if self.stage.constant_buffer_slot != -1 {
            gctx.bind_cbuffer(
                self.stage.constant_buffer_slot as u32,
                self.cbuffer.as_ref().map(|v| v.buffer().clone()),
                self.shader_stage,
            );
        }

        Ok(())
    }
}

#[repr(C)]
pub struct ScopeFrame {
    // pub time: Vec4,               // c0
    pub game_time: f32,
    pub render_time: f32,
    pub delta_game_time: f32,
    pub exposure_time: f32,

    // pub exposure: Vec4,           // c1
    pub exposure_scale: f32,
    pub exposure_illum_relative_glow: f32,
    pub exposure_scale_for_shading: f32,
    pub exposure_illum_relative: f32,

    pub random_seed_scales: Vec4, // c2
    pub overrides: Vec4,          // c3

    // Light related
    pub unk4: Vec4, // c4
    pub unk5: Vec4, // c5

    pub unk6: Vec4, // c6
    pub unk7: Vec4, // c7
}

impl Default for ScopeFrame {
    fn default() -> Self {
        ScopeFrame {
            game_time: Default::default(),
            render_time: Default::default(),
            delta_game_time: Default::default(),
            exposure_time: 1. / 60.,
            exposure_scale: 1.0, // 0.5674781799316406,
            exposure_illum_relative_glow: 23.386_974,
            exposure_scale_for_shading: 0.567_478_2,
            exposure_illum_relative: 1.461_685_9,
            random_seed_scales: Vec4::new(102.850_5, 102.048_53, 943.289_06, 187.406_77),
            overrides: Vec4::new(0.5, 0.5, 0.0, 0.0),
            unk4: Vec4::new(1.0, 1.0, 0.0, 1.0),
            unk5: Vec4::new(0.0, f32::NAN, 512.0, 0.0),
            unk6: Vec4::new(0.0, 1.0, 0.966_787_6, 0.0),
            unk7: Vec4::new(0.0, 0.5, 180.0, 0.0),
        }
    }
}

#[repr(C)]
#[derive(Clone, Default)]
pub struct ScopeInstances {
    pub mesh_offset: Vec3,
    pub mesh_scale: f32,
    pub uv_scale: f32,
    pub uv_offset: Vec2,
    pub max_color_index: u32,

    pub transforms: Vec<Mat4>,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ScopeRigidModel {
    pub mesh_to_world: Mat4,          // c0
    pub position_scale: Vec4,         // c4
    pub position_offset: Vec4,        // c5
    pub texcoord0_scale_offset: Vec4, // c6
    pub dynamic_sh_ao_values: Vec4,   // c7
    pub unk8: [Mat4; 8],              // c8
}

impl ScopeInstances {
    pub fn write(&self) -> Vec<u8> {
        let mut buffer = vec![];

        buffer
            .write_all(bytemuck::cast_slice(&[
                self.mesh_offset.x,
                self.mesh_offset.y,
                self.mesh_offset.z,
                self.mesh_scale,
                self.uv_scale,
                self.uv_offset.x,
                self.uv_offset.y,
                f32::from_bits(self.max_color_index),
            ]))
            .unwrap();

        buffer
            .write_all(bytemuck::cast_slice(&self.transforms))
            .unwrap();

        buffer
    }
}
