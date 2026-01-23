use std::sync::Arc;

use alkahest_core::job::{SCHEDULER, potassium::Priority};
use alkahest_data::tfx::{
    PipelineState, PrimitiveType, RenderStage, ShaderStage,
    common::AxisAlignedBBox,
    features::{
        dynamic::RenderStageSubscription,
        light::{SLight, SShadowingLight},
    },
};
use d3d11::dxgi;
use glam::{Mat4, Vec3, Vec4, Vec4Swizzles};
use itertools::Itertools;
use parking_lot::Mutex;
use tiger_pkg::TagHash;

use super::FeatureRenderer;
use crate::{
    Renderer,
    camera::Camera,
    gpu::command_list::DepthMode,
    renderer::surface::Surface,
    tfx::{
        externs::{self, DeferredLight, SimpleGeometry, VolumeFog},
        packet::CompactTransform,
        technique::Technique,
    },
    util::{geometry, threading::CommandListSetId},
};

struct LightRendererData {
    technique_lighting_apply: Technique,
    technique_lighting_apply_shadowing: Option<Technique>,
    technique_volumetrics: Option<Technique>,
    technique_volumetrics_shadowing: Option<Technique>,
    // technique_light_probe_apply: Technique,

    // TODO(cohae): This should be a shared resource (eg. a struct in the renderer that we can use instead of recreating it for every light/cubemap)
    vb: d3d11::Buffer,
    ib: d3d11::Buffer,
}

pub struct LightRenderer {
    data: Arc<LightRendererData>,

    local_to_world: glam::Mat4,
    light_space_transform: glam::Mat4,
    shadowmap_projection: glam::Mat4,
    bounds: Option<AxisAlignedBBox>,

    pub shadowmap: Option<Arc<Mutex<Option<Surface>>>>,
}

impl LightRenderer {
    pub fn new(
        renderer: &Renderer,
        light: &SLight,
        bounds: AxisAlignedBBox,
    ) -> anyhow::Result<Box<Self>> {
        Self::new_impl(
            renderer,
            light.technique_lighting_apply,
            TagHash::NONE,
            light.technique_volumetrics,
            TagHash::NONE,
            // light.technique_light_probe_apply,
            light.light_space_transform,
            Mat4::IDENTITY,
            Some(bounds),
        )
    }

    pub fn new_shadowing(
        renderer: &Renderer,
        light: &SShadowingLight,
        shadowmap_projection: Mat4,
    ) -> anyhow::Result<Box<Self>> {
        Self::new_impl(
            renderer,
            light.technique_lighting_apply,
            light.technique_lighting_apply_shadowing,
            light.technique_volumetrics,
            light.technique_volumetrics_shadowing,
            light.light_space_transform,
            shadowmap_projection,
            None,
        )
    }

    fn new_impl(
        renderer: &Renderer,
        technique_shading: TagHash,
        technique_shading_shadowing: TagHash,
        technique_volumetrics: TagHash,
        technique_volumetrics_shadowing: TagHash,
        // technique_light_probe: TagHash,
        light_space_transform: Mat4,
        shadowmap_projection: Mat4,
        bounds: Option<AxisAlignedBBox>,
    ) -> anyhow::Result<Box<Self>> {
        let vb = renderer.gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width(std::mem::size_of_val(geometry::CUBE_VERTICES) as u32)
                .usage(d3d11::Usage::Immutable)
                .bind_flags(d3d11::BindFlags::VERTEX_BUFFER)
                .build(),
            Some(bytemuck::cast_slice(geometry::CUBE_VERTICES)),
        )?;

        let ib = renderer.gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width(std::mem::size_of_val(geometry::CUBE_INDICES) as u32)
                .usage(d3d11::Usage::Immutable)
                .bind_flags(d3d11::BindFlags::INDEX_BUFFER)
                .build(),
            Some(bytemuck::cast_slice(geometry::CUBE_INDICES)),
        )?;

        Ok(Box::new(Self {
            data: Arc::new(LightRendererData {
                technique_lighting_apply: Technique::load(&renderer.gpu, technique_shading)?,
                technique_lighting_apply_shadowing: technique_shading_shadowing
                    .is_some()
                    .then(|| Technique::load(&renderer.gpu, technique_shading_shadowing))
                    .transpose()?,
                technique_volumetrics: technique_volumetrics
                    .is_some()
                    .then(|| Technique::load(&renderer.gpu, technique_volumetrics))
                    .transpose()?,
                technique_volumetrics_shadowing: technique_volumetrics_shadowing
                    .is_some()
                    .then(|| Technique::load(&renderer.gpu, technique_volumetrics_shadowing))
                    .transpose()?,
                // technique_light_probe_apply: Technique::load(&renderer.gpu, technique_light_probe)?,
                vb,
                ib,
            }),
            local_to_world: Mat4::IDENTITY,
            light_space_transform,
            shadowmap_projection,
            bounds,
            shadowmap: None,
        }))
    }
}

#[profiling::all_functions]
impl FeatureRenderer for LightRenderer {
    fn visibility_test(&mut self, camera: &Camera) -> bool {
        if let Some(ref bounds) = self.bounds {
            camera.is_visible(bounds)
        } else {
            false
        }
    }

    fn extract_and_prepare(
        &mut self,
        _renderer: &crate::Renderer,
        extracted_data: &dyn std::any::Any,
    ) {
        // TODO(cohae): lights shouldnt need to extract permutations at all
        let (obj_local_to_world, _permutation) = extracted_data
            .downcast_ref::<(CompactTransform, usize)>()
            .expect("Invalid extracted data type")
            .clone();

        self.local_to_world = obj_local_to_world.to_mat4();

        let local_to_world_scaled = self.local_to_world * self.light_space_transform;
        let points = geometry::CUBE_VERTICES
            .iter()
            .map(|&v| local_to_world_scaled.project_point3(v))
            .collect_vec();

        self.bounds = Some(AxisAlignedBBox::from_points(&points));
    }

    fn submit(&self, cmd: &mut crate::gpu::command_list::CommandList, stage: RenderStage) {
        if stage == RenderStage::LightProbeApply {
            return;
        }

        let shadowmap_projection = self.shadowmap_projection;

        let (_, transform_rot, transform_translation) =
            self.local_to_world.to_scale_rotation_translation();

        let forward = transform_rot * Vec3::X;
        let up = transform_rot * Vec3::Z;
        let transform_translation =
            transform_translation - Renderer::instance().externs.view.position();
        let transform_relative =
            Mat4::look_at_rh(transform_translation, transform_translation + forward, up);

        {
            let local_to_world_scaled = self.local_to_world * self.light_space_transform;
            let global_externs = Renderer::instance().externs.get();
            let is_camera_in_volume = self
                .bounds
                .as_ref()
                .is_none_or(|b| b.contains(global_externs.view.position()));

            if is_camera_in_volume {
                cmd.state = cmd
                    .state
                    .select(&PipelineState::new(None, Some(0), None, None));
            } else {
                cmd.state = cmd
                    .state
                    .select(&PipelineState::new(None, Some(30), None, None));
            }

            cmd.externs.simple_geometry = Some(Box::new(SimpleGeometry {
                local_to_world: global_externs.view.world_to_projective
                    * local_to_world_scaled
                    * if is_camera_in_volume {
                        Mat4::from_scale(Vec3::NEG_ONE)
                    } else {
                        Mat4::IDENTITY
                    },
            }));

            let view_translation_inverse_mat4 =
                Mat4::from_translation(-global_externs.view.position());
            let local_to_world_relative = view_translation_inverse_mat4 * self.local_to_world;

            let (min, max) = compute_light_bounds(self.light_space_transform);
            let light_local_to_world = compute_light_local_to_world(self.local_to_world, min, max);

            cmd.externs.deferred_light = Some(Box::new(DeferredLight {
                // unk40: local_to_world_relative.inverse().transpose(),
                unk40: (view_translation_inverse_mat4 * light_local_to_world).inverse(),
                unk80: local_to_world_relative,

                ..Default::default()
            }));

            cmd.externs.rigid_model = Some(Box::new(externs::RigidModel {
                local_to_world: light_local_to_world,
                ..Default::default()
            }));

            if stage == RenderStage::Volumetrics {
                let mut fog = VolumeFog::default();
                fog.unk00 = light_local_to_world.inverse();
                fog.unk40 = fog.unk00 * global_externs.view.target_pixel_to_world;
                fog.unka0 = (max - min).extend(1.);
                fog.unkb0 = 1.0;

                let p = fog
                    .unk00
                    .mul_vec4(global_externs.view.position().extend(1.0));
                let point_w_abs = (-p.wwww()).abs();
                fog.unk80 = Vec4::select(point_w_abs.cmpge(Vec4::splat(0.0001)), p / p.wwww(), p);
                // fog.unk80 = fog
                //     .unk00
                //     .project_point3(externs.view.position())
                //     .extend(1.0);

                if ((fog.unk80.x < -0.2) || (1.2 < fog.unk80.x))
                    || ((fog.unk80.y < -0.2) || (1.2 < fog.unk80.y))
                    || ((fog.unk80.z < -0.2) || (1.2 < fog.unk80.z))
                {
                    // cmd.state =
                    //     cmd.state
                    //         .select(&PipelineState::new(Some(0xf), Some(3), None, None));
                    fog.unkb4 = -1.0;
                } else {
                    // cmd.state = cmd
                    //     .state
                    //     .select(&PipelineState::new(Some(1), Some(2), None, None));
                    fog.unkb4 = 1.0;
                }

                cmd.externs.volume_fog = Some(fog.into());
            }
        }

        // TODO(cohae): The shadowmap lock is *extremely* messy, needs to be cleaned up
        let shadowmap_lock = self.shadowmap.as_ref().map(|v| v.lock());
        if Renderer::instance().settings().shadows
            && let Some(shadowmap2) = shadowmap_lock
            && let Some(shadowmap) = shadowmap2.as_ref()
        {
            // TODO(cohae): Unknown what this texture is supposed to be. VS loads the first pixel and uses it as multiplier for the shadowmap UVs
            Renderer::instance()
                .common
                .shadowmap_vs_t2
                .bind(cmd, 2, ShaderStage::Vertex);
            let existing_shadowmap = cmd
                .externs
                .deferred_shadow
                .as_ref()
                .cloned()
                .unwrap_or_default();

            cmd.externs.deferred_shadow = Some(
                externs::DeferredShadow {
                    shadow_depthmap: shadowmap.srv(0).cloned().into(),
                    resolution_width: shadowmap.resolution().0 as f32,
                    resolution_height: shadowmap.resolution().1 as f32,
                    // unkc0: shadowmap.camera_to_projective * transform_relative.view_matrix(),
                    unkc0: shadowmap_projection * transform_relative,
                    unk180: 1.0,
                    // unk180: renderer.settings.shadow_quality.pcf_samples() as u8 as f32,
                    ..*existing_shadowmap
                }
                .into(),
            );

            if stage == RenderStage::Volumetrics {
                if let Some(technique) = self
                    .data
                    .technique_volumetrics_shadowing
                    .as_ref()
                    .or(self.data.technique_volumetrics.as_ref())
                {
                    technique.bind(cmd).unwrap();
                } else {
                    return;
                }
            } else {
                // self.data.technique_lighting_apply.bind(cmd).unwrap();
                self.data
                    .technique_lighting_apply_shadowing
                    .as_ref()
                    .unwrap_or(&self.data.technique_lighting_apply)
                    .bind(cmd)
                    .unwrap();
            }
        } else if stage == RenderStage::Volumetrics {
            if let Some(ref technique) = self.data.technique_volumetrics {
                technique.bind(cmd).unwrap();
            } else {
                return;
            }
        } else {
            self.data.technique_lighting_apply.bind(cmd).unwrap();
        }

        cmd.set_input_topology(PrimitiveType::Triangles);
        cmd.set_input_layout(1); // float3 v0 : POSITION0, // Format DXGI_FORMAT_R32G32B32_FLOAT size 12

        cmd.input_assembler_set_index_buffer(&self.data.ib, dxgi::Format::R16Uint, 0);
        cmd.input_assembler_set_vertex_buffers(0, &[Some(&self.data.vb)], Some(&[12]), Some(&[0]))
            .unwrap();

        cmd.draw_indexed(geometry::CUBE_INDICES.len() as u32, 0, 0);
        cmd.flush_states();
    }

    fn submit_parallel(
        &self,
        renderer: &std::sync::Arc<Renderer>,
        set: CommandListSetId,
        stage: RenderStage,
        jobs: &mut Vec<alkahest_core::job::potassium::JobHandle>,
    ) {
        // let (scale, _rotation, _translation) =
        //     self.local_to_world.to_scale_rotation_translation();

        let pool_clone = renderer.cmd_pool.clone();
        let light_space_transform = self.light_space_transform;
        let local_to_world = self.local_to_world;
        let shadowmap_projection = self.shadowmap_projection;
        let local_to_world_scaled = local_to_world * light_space_transform;
        let data = self.data.clone();
        let bounds = self.bounds.clone();

        let (_, transform_rot, transform_translation) =
            self.local_to_world.to_scale_rotation_translation();

        let forward = transform_rot * Vec3::X;
        let up = transform_rot * Vec3::Z;
        let transform_translation = transform_translation - renderer.externs.view.position();
        let transform_relative =
            Mat4::look_at_rh(transform_translation, transform_translation + forward, up);

        let shadowmap = self.shadowmap.clone();

        let job = SCHEDULER
            .job_builder("light_render")
            .priority(Priority::Medium)
            .spawn(move || {
                let cmd = pool_clone.get_command_list(set);
                {
                    let externs = Renderer::instance().externs.get();

                    let is_camera_in_volume = bounds
                        .as_ref()
                        .is_none_or(|b| b.contains(externs.view.position()));

                    if is_camera_in_volume {
                        cmd.state =
                            cmd.state
                                .select(&PipelineState::new(None, Some(0), None, None));
                    } else {
                        cmd.state =
                            cmd.state
                                .select(&PipelineState::new(None, Some(30), None, None));
                    }

                    cmd.externs.simple_geometry = Some(Box::new(SimpleGeometry {
                        local_to_world: externs.view.world_to_projective
                            * local_to_world_scaled
                            * if is_camera_in_volume {
                                Mat4::from_scale(Vec3::NEG_ONE)
                            } else {
                                Mat4::IDENTITY
                            },
                    }));

                    let view_translation_inverse_mat4 =
                        Mat4::from_translation(-externs.view.position());
                    let local_to_world_relative = view_translation_inverse_mat4 * local_to_world;

                    let (min, max) = compute_light_bounds(light_space_transform);
                    let light_local_to_world =
                        compute_light_local_to_world(local_to_world, min, max);

                    cmd.externs.deferred_light = Some(Box::new(DeferredLight {
                        // unk40: local_to_world_relative.inverse().transpose(),
                        unk40: (view_translation_inverse_mat4 * light_local_to_world).inverse(),
                        unk80: local_to_world_relative,

                        ..Default::default()
                    }));

                    cmd.externs.rigid_model = Some(Box::new(externs::RigidModel {
                        local_to_world: light_local_to_world,
                        ..Default::default()
                    }));
                }

                // TODO(cohae): The shadowmap lock is *extremely* messy, needs to be cleaned up
                let shadowmap_lock = shadowmap.as_ref().map(|v| v.lock());
                if Renderer::instance().settings().shadows
                    && let Some(shadowmap2) = shadowmap_lock
                    && let Some(shadowmap) = shadowmap2.as_ref()
                {
                    // TODO(cohae): Unknown what this texture is supposed to be. VS loads the first pixel and uses it as multiplier for the shadowmap UVs
                    Renderer::instance()
                        .common
                        .shadowmap_vs_t2
                        .bind(cmd, 2, ShaderStage::Vertex);
                    let existing_shadowmap = cmd
                        .externs
                        .deferred_shadow
                        .as_ref()
                        .cloned()
                        .unwrap_or_default();

                    debug_assert!(shadowmap.srv(0).is_some());
                    cmd.externs.deferred_shadow = Some(
                        externs::DeferredShadow {
                            shadow_depthmap: shadowmap.srv(0).cloned().into(),
                            resolution_width: shadowmap.resolution().0 as f32,
                            resolution_height: shadowmap.resolution().1 as f32,
                            // unkc0: shadowmap.camera_to_projective * transform_relative.view_matrix(),
                            unkc0: shadowmap_projection * transform_relative,
                            unk180: 2.0,
                            // unk180: renderer.settings.shadow_quality.pcf_samples() as u8 as f32,
                            ..*existing_shadowmap
                        }
                        .into(),
                    );

                    if stage == RenderStage::Volumetrics {
                        if let Some(technique) = data
                            .technique_volumetrics_shadowing
                            .as_ref()
                            .or(data.technique_volumetrics.as_ref())
                        {
                            technique.bind(cmd).unwrap();
                        } else {
                            return;
                        }
                    } else {
                        data.technique_lighting_apply_shadowing
                            .as_ref()
                            .unwrap_or(&data.technique_lighting_apply)
                            .bind(cmd)
                            .unwrap();
                    }
                } else if stage == RenderStage::Volumetrics {
                    if let Some(ref technique) = data.technique_volumetrics {
                        technique.bind(cmd).unwrap();
                    } else {
                        return;
                    }
                } else {
                    data.technique_lighting_apply.bind(cmd).unwrap();
                }

                cmd.set_input_topology(PrimitiveType::Triangles);
                cmd.set_input_layout(1); // float3 v0 : POSITION0, // Format DXGI_FORMAT_R32G32B32_FLOAT size 12

                cmd.input_assembler_set_index_buffer(&data.ib, dxgi::Format::R16Uint, 0);
                cmd.input_assembler_set_vertex_buffers(
                    0,
                    &[Some(&data.vb)],
                    Some(&[12]),
                    Some(&[0]),
                )
                .unwrap();

                cmd.draw_indexed(geometry::CUBE_INDICES.len() as u32, 0, 0);
            });

        jobs.push(job);
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        RenderStageSubscription::LIGHTING_APPLY
            | RenderStageSubscription::LIGHT_PROBE_APPLY
            | RenderStageSubscription::VOLUMETRICS
    }
}

fn compute_light_bounds(light_space_transform: Mat4) -> (Vec3, Vec3) {
    let mut points = [
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(1.0, 1.0, 1.0),
    ];

    for point in &mut points {
        let p = light_space_transform.mul_vec4(point.extend(1.0));
        let point_w_abs = (-p.wwww()).abs();
        *point = Vec4::select(
            point_w_abs.cmpge(Vec4::splat(0.0001)),
            p / p.wwww(),
            Vec4::W,
        )
        .truncate();
    }

    points
        .iter()
        .fold((Vec3::MAX, Vec3::MIN), |(min, max), &point| {
            (min.min(point), max.max(point))
        })
}

fn compute_light_local_to_world(node_local_to_world: Mat4, min: Vec3, max: Vec3) -> Mat4 {
    let bounds_center = min.midpoint(max);
    let bounds_half_extents = (max - min) / 2.0;

    // First matrix operation ("mat"):
    // Each column is computed by scaling one of node_local_to_world’s axes by the corresponding component of bounds_half_extents,
    // except for the w-axis which is a linear combination of the x, y, and z axes plus the original w-axis.
    let mat = Mat4 {
        x_axis: node_local_to_world.x_axis * bounds_half_extents.x,
        y_axis: node_local_to_world.y_axis * bounds_half_extents.y,
        z_axis: node_local_to_world.z_axis * bounds_half_extents.z,
        w_axis: node_local_to_world.x_axis * bounds_center.x
            + node_local_to_world.y_axis * bounds_center.y
            + node_local_to_world.z_axis * bounds_center.z
            + node_local_to_world.w_axis,
    };

    // Second matrix operation ("mat_scaled"):
    // Scale the x, y, and z axes by 2, and subtract all three from the w-axis.
    let mat_scaled = Mat4 {
        x_axis: mat.x_axis * 2.0,
        y_axis: mat.y_axis * 2.0,
        z_axis: mat.z_axis * 2.0,
        w_axis: mat.w_axis - mat.x_axis - mat.y_axis - mat.z_axis,
    };

    // Third matrix operation (computing light_local_to_world):
    // Rearrange the columns of mat_scaled: swap the x and z axes, leaving y and w unchanged.

    Mat4 {
        x_axis: mat_scaled.z_axis,
        y_axis: mat_scaled.y_axis,
        z_axis: mat_scaled.x_axis,
        w_axis: mat_scaled.w_axis,
    }
}
