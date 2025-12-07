// pub mod bloom;
pub mod buffers;
pub mod gbuffer;
pub mod lighting;
pub mod lowlevel;
pub mod transparent;
// pub mod water;

use std::fmt::Debug;

use alkahest_core::convar::ConVars;
use alkahest_data::tfx::{FeatureRendererSubscription, PipelineState, ShaderStage};
use glam::{vec4, Mat4, Vec3, Vec4};

use super::Renderer;
use crate::{
    camera::Camera,
    cmd_event_span,
    gpu::command_list::CommandList,
    tfx::{
        externs::{self, GlobalLighting},
        scope::TempFrameScope,
        view::View,
    },
};

impl Renderer {
    pub fn submit_world(
        &self,
        cmd: &mut CommandList,
        view: &View,
        delta_time: f32,
        debug_pipeline: Option<DebugPipeline>,
    ) {
        cmd_event_span!(cmd, "submit_world");
        let _gpuspan = self.profiler.scope(cmd, "submit_world");

        *self.surfaces.write() = view.surfaces.clone();
        view.surfaces.resize_surfaces(view.resolution);

        self.active_feature_renderers
            .store(self.calculate_active_feature_renderers());

        let gpu = &self.gpu;

        self.prepare_externs(
            cmd,
            view,
            self.start_time.elapsed().as_secs_f32(),
            delta_time,
        );

        self.globals.scopes.view.bind(cmd).unwrap();

        self.submit_gbuffer_generation(cmd, view);

        if matches!(
            debug_pipeline,
            Some(DebugPipeline::DeferredShading)
                | Some(DebugPipeline::DeferredShadingNoSun)
                | Some(DebugPipeline::LightDiffuse)
                | Some(DebugPipeline::LightSpecular)
        ) {
            self.submit_lighting(cmd, view);
        }

        self.clear_surface(cmd, view.shading_result, [0., 0., 0., 1.0]);
        self.bind_surfaces(cmd, &[view.shading_result], None);
        cmd.output_merger_set_depth_stencil_state(None, 0);

        cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
        if ConVars::get_flag("render.global_lighting") {
            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.global_lighting_and_shading_gel,
                "global_lighting_and_shading_gel",
            );
        } else {
            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.deferred_shading,
                "deferred_shading",
            );
        }

        // view.shading_result_read
        //     .lock()
        //     .update(cmd, view.surfaces.get(view.shading_result));

        // self.submit_transparent(cmd);

        // self.shading_result_read
        //     .lock()
        //     .update(&cmd, view.surfaces.get(self.shading_result));

        // self.submit_water(cmd);

        // if ConVars::get_flag("render.feature.volumetrics") {
        //     self.apply_volume_fog(cmd);
        // }

        // self.submit_bloom(cmd);

        {
            // view.shading_result_read
            //     .lock()
            //     .update(cmd, view.surfaces.get(view.shading_result));
            view.surfaces.get(view.shading_result).bind_single(cmd);
            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            // cmd.flush_states();
            self.execute_global_pipeline(
                cmd,
                self.globals
                    .pipelines
                    // .screen_area_global_lut3d_no_tonemap,
                    .get_specialized_lut3d_pipeline(true, false, false),
                "screen_area_global_lut3d",
            );
        }
        // {
        //     cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
        //     cmd.flush_states();
        //     cmd.rasterizer_set_viewports(&[d3d11::Viewport::builder()
        //         .width(gpu.swapchain_resolution().0 as f32)
        //         .height(gpu.swapchain_resolution().1 as f32)
        //         .build()]);
        //     cmd.vertex_set_shader(Some(&self.common.blit_vs));
        //     cmd.pixel_set_shader(Some(&self.common.blit_ps));
        //     cmd.set_input_topology(alkahest_data::tfx::PrimitiveType::TriangleStrip);
        //     cmd.clear_render_target_view(&gpu.acquire_rtv(), &[0., 0., 0., 1.0]);
        //     cmd.output_merger_set_render_targets(&[Some(gpu.acquire_rtv())], None);
        //     let srv_shading_result = view.surfaces.get(self.shading_result).srv.clone();
        //     cmd.pixel_set_shader_resources(0, &[srv_shading_result]);
        //     cmd.draw(4, 0);
        // }

        let output = view.surfaces.get(view.output);
        self.profiler.scope(cmd, "debug_view").span(|| {
            cmd.rasterizer_set_viewports(&[d3d11::Viewport::builder()
                .width(output.resolution().0 as f32)
                .height(output.resolution().1 as f32)
                .build()]);
            cmd.clear_render_target_view(output.rtv.as_ref().unwrap(), &[0., 0., 0., 1.0]);
            cmd.output_merger_set_render_targets(std::slice::from_ref(&output.rtv.as_ref()), None);

            if let Some(debug_pipeline) = debug_pipeline {
                let p = &self.globals.pipelines;
                let technique = match debug_pipeline {
                    DebugPipeline::DeferredShading => &p.global_lighting_and_shading,
                    DebugPipeline::DeferredShadingNoSun => &p.deferred_shading_no_atm,
                    DebugPipeline::Albedo => &p.debug_source_color,
                    DebugPipeline::Smoothness => &p.debug_specular_smoothness,
                    DebugPipeline::Metalness => &p.debug_metalness,
                    DebugPipeline::AmbientOcclusion => &p.debug_texture_ao,
                    DebugPipeline::Emission => &p.debug_emissive,
                    DebugPipeline::Transmission => &p.debug_transmission,
                    DebugPipeline::Overcoat => &p.debug_colored_overcoat_id,
                    DebugPipeline::DepthEdges => &p.debug_depth_edges,
                    DebugPipeline::WorldNormal => &p.debug_world_normal,
                    DebugPipeline::LightDiffuse => &p.debug_diffuse_light,
                    DebugPipeline::LightSpecular => &p.debug_specular_light,
                };

                self.execute_global_pipeline(cmd, technique, &format!("{debug_pipeline:?}"));
            } else {
                let sun_light_direction = self
                    .externs
                    .get_global_channel_by_name("sun_light_direction");
                self.debug_cbuffer
                    .write(
                        cmd,
                        &Mat4 {
                            x_axis: sun_light_direction,
                            ..Default::default()
                        },
                    )
                    .ok();
                self.debug_cbuffer.bind(cmd, ShaderStage::Pixel, 0);

                cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
                cmd.flush_states();
                cmd.vertex_set_shader(Some(&self.debug_vs));
                cmd.pixel_set_shader(Some(&self.debug_ps));
                cmd.set_input_topology(alkahest_data::tfx::PrimitiveType::TriangleStrip);
                cmd.pixel_set_shader_resources(
                    0,
                    &[
                        view.surfaces.get(view.gbuffers.albedo).srv.as_ref(),
                        view.surfaces.get(view.gbuffers.normal).srv.as_ref(),
                        view.surfaces.get(view.gbuffers.third).srv.as_ref(),
                        Some(&view.gbuffers.depth_proxy.lock().srv),
                    ],
                );
                cmd.draw(4, 0);
            }
        });

        self.submit_transparent(cmd, view);

        {
            profiling::scope!("prepare/submit immediate geometry");
            let _gpuspan = self.profiler.scope(cmd, "immediate_geometry");
            cmd.output_merger_set_render_targets(
                std::slice::from_ref(&output.rtv.as_ref()),
                view.surfaces.get(view.gbuffers.depth).dsv.as_ref(),
            );
            cmd.state = PipelineState::new(Some(0), Some(2), Some(2), Some(0));
            cmd.flush_states();
            self.immediate.lock().prepare(gpu);
            self.immediate.lock().submit(cmd);
        }
    }

    fn prepare_externs(
        &self,
        cmd: &mut CommandList,
        view: &View,
        render_time: f32,
        delta_time: f32,
    ) {
        let fb_res = view.surfaces.framebuffer_resolution();

        // let cam_view = Mat4::from_cols(
        //     [-0.962532818, -0.027713167, -0.269745320, 0.000000000].into(),
        //     [-0.271165162, 0.098371163, 0.957492828, 0.000000000].into(),
        //     [0.000000000, 0.994763792, -0.102200322, 0.000000000].into(),
        //     [15.103929520, -31.395317078, -47.990650177, 1.000000000].into(),
        // );
        // let cam_proj = Mat4::from_cols(
        //     [0.827271998, 0.000000000, 0.000000000, 0.000000000].into(),
        //     [0.000000000, 1.470705628, 0.000000000, 0.000000000].into(),
        //     [0.000000000, 0.000000000, 0.000002623, -1.000000000].into(),
        //     [0.000000000, 0.000000000, 0.150000393, 0.000000000].into(),
        // );

        let ext = self.externs.get_mut();
        ext.view
            .update(view.world_to_camera, view.camera_to_projective, fb_res);

        *ext.frame = externs::Frame {
            game_time: render_time, //self.start_time.elapsed().as_secs_f32();
            render_time,            //self.start_time.elapsed().as_secs_f32();
            delta_game_time: delta_time,
            exposure_time: 0.016666668,
            // exposure_scale: 7.71489,
            exposure_scale: 2.0,
            exposure_illum_relative: 0.25438,
            ..*ext.frame.clone()
        };

        // TODO(cohae): Reconfirm the offset of iridescence lookup
        // let irr_lookup = &self.globals.textures.iridescence_lookup;
        // ext.frame.iridescence_lookup = irr_lookup.view.clone().into();

        let near = Camera::NEAR;
        let far = Camera::FAR;
        ext.deferred.depth_constants = Vec4::new(
            1.0 / far,
            (far - near) / (far * near),
            0.00000000,
            0.00000000,
        );

        // ext.deferred.gbuffer_resolution_scale_offset =
        //     Vec4::new(fb_res.0 as f32, fb_res.1 as f32, 0.0, 0.0);
        ext.deferred.deferred_depth = view.gbuffers.depth_proxy.lock().srv.clone().into();
        ext.deferred.deferred_rt0 = view.gbuffers.albedo.into();
        ext.deferred.deferred_rt1 = view.gbuffers.normal.into();
        ext.deferred.deferred_rt2 = view.gbuffers.third.into();

        ext.deferred.light_diffuse = view.lighting.light_diffuse.into();
        ext.deferred.light_specular = view.lighting.light_specular.into();
        ext.deferred.light_specular_ibl = view.lighting.light_specular_ibl.into();

        // ext.deferred.sky_hemisphere_mips = self.common.temporary_sky_hemisphere.view.clone().into();

        ext.decal.depth_read = view.gbuffers.depth_proxy.lock().srv.clone().into();
        ext.decal.normals_read = view.gbuffers.normal_read.into();
        ext.decal.depth_constants = ext.deferred.depth_constants;
        ext.decal.unk30 = Vec4::new(fb_res.0 as f32, fb_res.1 as f32, 0.0, 0.0);

        *ext.global_lighting = GlobalLighting {
            unk08: self.gpu.placeholder_white.view.clone().into(),
            unk10: ext.get_global_channel_by_name("sun_color")
                * ext.get_global_channel_by_name("sun_intensity").x,
            unk30: ext.get_global_channel_by_name("sun_light_direction"),
            unk50: ext.get_global_channel_by_name("sun_ambient_direction"),
            unk70: ext.get_global_channel_by_name("up_ambient_color")
                * ext.get_global_channel_by_name("up_ambient_intensity").x,
            unk80: ext.get_global_channel_by_name("down_ambient_color")
                * ext.get_global_channel_by_name("down_ambient_intensity").x,
            unk90: ext.get_global_channel_by_name("up_ambient_sharpness").x,
            unk94: ext.get_global_channel_by_name("down_ambient_sharpness").x,
            unka0: vec4(0.01, 0.01, -0.5, -0.5),
            unkb0: vec4(0.02, -2.0, 0.0, 0.0),
            // unkd0: vec4(f32::NAN, f32::NAN, 0.5, 0.5),
            unkc0: vec4(0.00333, -2.33333, 0.00, 0.00),
            ..Default::default()
        };

        // ext.shadow_mask.unk00 = self.gpu.placeholder_white.view.clone().into();
        // ext.shadow_mask.unk08 = self.lighting.ssao.into();
        // ext.shadow_mask.unk10 = self.gbuffers.uber_depth_half.into();

        // if let Some(vao_srv) = view.surfaces.get(self.lighting.vertex_ao).srv.clone() {
        //     ext.cubemaps.vertex_ao = vao_srv.into();
        // }

        // ext.atmosphere.unk38 = self.common.temporary_depth_lookup.view.clone().into();
        // ext.atmosphere.unk88 = self.common.temporary_atmos.view.clone().into();

        // ext.screen_area = ScreenArea {
        //     unk00: self.shading_result_read.lock().srv.clone().into(),
        //     unk10: self.common.default_lut.view.clone().into(), // LUT
        //     unk18: self.common.temporary_bloom.view.clone().into(), // bloom
        //     unk20: self.lighting.distortion.into(),             // distortion
        //     unk28: TextureView::None,                           // health overlay
        //     unk30: self.common.temporary_vignette.view.clone().into(), // vignette
        //     unk48: 0.9968,
        //     unk70: Vec4::new(0.13281, 0.23611, 0.00, 0.00), // distortion related
        //     unkd0: Vec4::new(0.3, 0.5, 0.0, 0.02),
        //     unkc0: 0.05,
        //     unke0: Vec4::new(0.3, 0.5, 0.0, 0.5),
        //     ..Default::default()
        // };

        // let depth_res = view.surfaces.get(self.gbuffers.depth).resolution();
        // ext.uber_depth = UberDepth {
        //     original_depth: self.gbuffers.depth_proxy.lock().srv.clone().into(),
        //     unk30: self.gbuffers.uber_depth_half.into(),
        //     unk40: self.gbuffers.uber_depth_quarter.into(),
        //     unk50: ext.deferred.depth_constants,
        //     unk70: Vec4::new(0.0, 0.0, depth_res.0 as f32, depth_res.1 as f32),
        //     ..Default::default()
        // };

        *ext.transparent = externs::Transparent {
            // unk00: self.gpu.placeholder_white.view.clone().into(),
            // unk10: self.gpu.placeholder_white.view.clone().into(),
            // unk20: self.gpu.placeholder_white.view.clone().into(),
            // unk48: self.gpu.placeholder_white.view.clone().into(),
            // unk50: self.gpu.placeholder_white.view.clone().into(),
            // unk78: self.gpu.placeholder_white.view.clone().into(),
            // unk38: self.common.temporary_atmos.view.clone().into(),
            unk80: vec4(0.22882, 0.00, 1.00, 45.00),
            unk90: vec4(0.00, 0.00, 1.17485, 2.86546),
            unka0: vec4(0.00, 0.00, 2.10913, 5.14044),
            unkb0: vec4(0.00, 0.00, 3.46762, 8.41667),
            unkc0: vec4(0.00, 0.00, 0.00, 0.00),
            unkd0: vec4(0.00, 0.00, 0.00, 0.00),
            unke0: vec4(0.00, 0.00, 0.00, 0.00),
            ..Default::default()
        };

        // TODO(cohae): use the actual frame scope instead of the temporary `frame_scope`
        self.globals.scopes.frame.bind(cmd).unwrap();
        let _ = self.frame_scope.write(
            cmd,
            &TempFrameScope {
                game_time: ext.frame.game_time, //self.start_time.elapsed().as_secs_f32(),
                render_time: ext.frame.render_time, //self.start_time.elapsed().as_secs_f32(),
                delta_game_time: ext.frame.delta_game_time,
                exposure_time: ext.frame.exposure_time,

                // exposure_scale: 1.,
                // exposure_illum_relative_glow: 1.,
                // exposure_illum_relative: 1.,
                // exposure_scale_for_shading: 1.,
                exposure_scale: ext.frame.exposure_scale,
                exposure_illum_relative_glow: ext.frame.exposure_illum_relative * 16.0,
                exposure_scale_for_shading: ext.frame.exposure_scale,
                exposure_illum_relative: ext.frame.exposure_illum_relative,
                random_seed_scales: Vec4::new(
                    (render_time * 60.0 + 33.75) * 1.258699,
                    (render_time * 60.0 + 60.0) * 0.9583125,
                    (render_time * 60.0 + 60.0) * 8.789123,
                    (render_time * 60.0 + 33.75) * 2.311535,
                ),
                unk3: Vec4::new(0.5, 0.5, 0.0, 0.0),
                unk4: Vec4::new(1.0, 1.0, 0.0, 1.0),
                unk5: Vec4::new(0.00, -f32::NAN, 512.00, 0.00),
                unk6: Vec4::ONE,
            },
        );
        self.frame_scope.bind(cmd, ShaderStage::Vertex, 13);
        self.frame_scope.bind(cmd, ShaderStage::Pixel, 13);
    }

    fn calculate_active_feature_renderers(&self) -> FeatureRendererSubscription {
        let mut sub = FeatureRendererSubscription::all();
        macro_rules! remove_feature_if_unset {
            ($convar:expr, $flag:ident) => {
                if !ConVars::get_flag(concat!("render.feature.", $convar)) {
                    sub.remove(FeatureRendererSubscription::$flag);
                }
            };
        }

        remove_feature_if_unset!("static_objects", STATIC_OBJECTS);
        remove_feature_if_unset!("terrain_patches", TERRAIN_PATCH);
        remove_feature_if_unset!("rigid_objects", RIGID_OBJECT);
        remove_feature_if_unset!("chunked_lights", CHUNKED_LIGHTS);
        remove_feature_if_unset!("deferred_lights", DEFERRED_LIGHTS);
        remove_feature_if_unset!("sky_transparent", SKY_TRANSPARENT);
        remove_feature_if_unset!("decals", DECALS);
        remove_feature_if_unset!("dynamic_decals", DYNAMIC_DECALS);
        remove_feature_if_unset!("road_decals", ROAD_DECALS);
        remove_feature_if_unset!("water", WATER);
        remove_feature_if_unset!("volumetrics", VOLUMETRICS);
        remove_feature_if_unset!("cubemaps", CUBEMAPS);

        sub
    }
}

#[derive(Debug, PartialEq)]
pub enum DebugPipeline {
    DeferredShading,
    DeferredShadingNoSun,

    Albedo,
    Smoothness,
    Metalness,
    AmbientOcclusion,
    Emission,
    Transmission,
    Overcoat,

    DepthEdges,
    WorldNormal,

    LightDiffuse,
    LightSpecular,
}
