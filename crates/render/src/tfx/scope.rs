use std::sync::Arc;

use alkahest_data::tfx::{
    ShaderStage,
    scope::{SScope, SScopeStage},
};
use glam::{Mat4, Vec3A, Vec4};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use super::dynamic_constants::DynamicConstants;
use crate::{Gpu, gpu::command_list::CommandList};

pub struct Scope {
    scope: SScope,

    pub stage_pixel: Option<Box<ScopeStage>>,
    pub stage_vertex: Option<Box<ScopeStage>>,
    pub stage_geometry: Option<Box<ScopeStage>>,
    pub stage_compute: Option<Box<ScopeStage>>,
}

impl Scope {
    #[profiling::function]
    pub fn load(gpu: &Arc<Gpu>, hash: TagHash) -> anyhow::Result<Self> {
        let scope: SScope = package_manager().read_tag_struct(hash)?;

        let stage_vertex = if scope.stage_vertex.constants.constant_buffer_slot != -1 {
            Some(ScopeStage::load(
                gpu,
                &scope.stage_vertex,
                ShaderStage::Vertex,
            )?)
        } else {
            None
        };

        let stage_pixel = if scope.stage_pixel.constants.constant_buffer_slot != -1 {
            Some(ScopeStage::load(
                gpu,
                &scope.stage_pixel,
                ShaderStage::Pixel,
            )?)
        } else {
            None
        };

        let stage_geometry = if scope.stage_geometry.constants.constant_buffer_slot != -1 {
            Some(ScopeStage::load(
                gpu,
                &scope.stage_geometry,
                ShaderStage::Geometry,
            )?)
        } else {
            None
        };

        let stage_compute = if scope.stage_compute.constants.constant_buffer_slot != -1 {
            Some(ScopeStage::load(
                gpu,
                &scope.stage_compute,
                ShaderStage::Compute,
            )?)
        } else {
            None
        };

        Ok(Self {
            scope,
            stage_pixel,
            stage_vertex,
            stage_geometry,
            stage_compute,
        })
    }

    #[tracing::instrument(skip(self, cmd), fields(scope = %self.scope.name.0))]
    pub fn bind(&self, cmd: &mut CommandList) -> anyhow::Result<()> {
        // let _s = info_span!("Bind scope", scope = %self.scope.name.0).entered();
        if let Some(stage) = &self.stage_vertex {
            stage.bind(cmd)?;
        }

        if let Some(stage) = &self.stage_pixel {
            stage.bind(cmd)?;
        }

        if let Some(stage) = &self.stage_geometry {
            stage.bind(cmd)?;
        }

        if let Some(stage) = &self.stage_compute {
            stage.bind(cmd)?;
        }

        Ok(())
    }

    pub fn vertex_slot(&self) -> i32 {
        self.scope.stage_vertex.constants.constant_buffer_slot
    }
}

pub struct ScopeStage {
    shader_stage: ShaderStage,
    constants: DynamicConstants,
}

impl ScopeStage {
    pub fn load(
        gpu: &Arc<Gpu>,
        stage: &SScopeStage,
        shader_stage: ShaderStage,
    ) -> anyhow::Result<Box<ScopeStage>> {
        let constants = DynamicConstants::load(gpu, &stage.constants)?;

        Ok(Box::new(Self {
            constants,
            shader_stage,
        }))
    }

    pub fn bind(&self, cmd: &mut CommandList) -> anyhow::Result<()> {
        self.constants.bind(cmd, self.shader_stage, None)
    }
}

// TODO(cohae): We need to somehow be able to hook into the existing scopes in order to update the ones that are not populated by expressions
#[repr(C)]
pub struct TempFrameScope {
    pub game_time: f32,
    pub render_time: f32,
    pub delta_game_time: f32,
    pub exposure_time: f32,

    pub exposure_scale: f32,
    pub exposure_illum_relative_glow: f32,
    pub exposure_scale_for_shading: f32,
    pub exposure_illum_relative: f32,

    pub random_seed_scales: Vec4,
    pub unk3: Vec4,
    pub unk4: Vec4,
    pub unk5: Vec4,
    pub unk6: Vec4,
}

#[repr(C)]
pub struct CascadeScope {
    pub target_pixel_to_world: Mat4,
    pub camera_to_projective: Mat4,
    pub world_to_camera: Mat4,
    pub world_to_cascade: Mat4,
    pub light_dir: Vec3A,
    pub plane_distance: f32,
}
