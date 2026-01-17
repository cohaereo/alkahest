use std::sync::Arc;

use alkahest_data::tfx::{
    ShaderStage,
    scope::{SScope, SScopeStage},
};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3A, Vec4};
use parking_lot::RwLock;
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

    pub fn write_initial_constants(
        &self,
        cmd: &mut CommandList,
        new_constants: &[Vec4],
    ) -> anyhow::Result<()> {
        if let Some(stage) = &self.stage_vertex {
            stage.write_initial_constants(cmd, new_constants)?;
        }
        if let Some(stage) = &self.stage_pixel {
            stage.write_initial_constants(cmd, new_constants)?;
        }
        if let Some(stage) = &self.stage_geometry {
            stage.write_initial_constants(cmd, new_constants)?;
        }
        if let Some(stage) = &self.stage_compute {
            stage.write_initial_constants(cmd, new_constants)?;
        }
        Ok(())
    }
}

pub struct ScopeStage {
    shader_stage: ShaderStage,
    constants: RwLock<DynamicConstants>,
}

impl ScopeStage {
    pub fn load(
        gpu: &Arc<Gpu>,
        stage: &SScopeStage,
        shader_stage: ShaderStage,
    ) -> anyhow::Result<Box<ScopeStage>> {
        let constants = DynamicConstants::load(gpu, &stage.constants)?;

        Ok(Box::new(Self {
            constants: RwLock::new(constants),
            shader_stage,
        }))
    }

    pub fn bind(&self, cmd: &mut CommandList) -> anyhow::Result<()> {
        self.constants.read().bind(cmd, self.shader_stage, None)
    }

    /// Overrides the initial constants with the given values.
    ///
    /// Returns an error if the given slice is bigger than the cbuffer. If the slice is smaller, the remaining values are left unchanged.
    pub fn write_initial_constants(
        &self,
        cmd: &mut CommandList,
        new_constants: &[Vec4],
    ) -> anyhow::Result<()> {
        let Some(cbuffer_size) = self
            .constants
            .read()
            .cbuffer
            .as_ref()
            .map(|c| c.size() / 16)
        else {
            // Not all stages have a cbuffer
            return Ok(());
        };

        let mut constants = self.constants.write();
        {
            let initial_constants = &mut constants.initial_constants;
            if initial_constants.len() < cbuffer_size {
                initial_constants.resize(cbuffer_size, Vec4::ZERO);
            }

            if new_constants.len() > initial_constants.len() {
                return Err(anyhow::anyhow!("Given slice is bigger than the cbuffer"));
            }
            initial_constants[..new_constants.len()].copy_from_slice(new_constants);
        }

        // Initial constants aren't normally copied to the cbuffer unless there's expression bytecode, so we need to copy them manually
        if (constants.bytecode.is_empty() || !constants.writes_cbuffer)
            && let Some(cbuffer) = constants.cbuffer.as_ref()
        {
            unsafe {
                cbuffer
                    .write_array(cmd, new_constants)
                    .expect("Failed to write new initial constants to cbuffer");
            }
        }

        Ok(())
    }
}

// TODO(cohae): We need to somehow be able to hook into the existing scopes in order to update the ones that are not populated by expressions
#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct FrameScope {
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

impl FrameScope {
    pub fn to_array(&self) -> &[Vec4; 7] {
        bytemuck::cast_ref::<FrameScope, [Vec4; _]>(self)
    }
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
