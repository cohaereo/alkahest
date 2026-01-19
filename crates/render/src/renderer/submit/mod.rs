pub mod atmosphere;
pub mod bloom;
pub mod buffers;
pub mod gbuffer;
pub mod geometry;
pub mod lighting;
pub mod lowlevel;
pub mod sun_shadows;
pub mod transparent;
pub mod water;

use std::{fmt::Debug, sync::Arc};

use alkahest_core::convar::ConVars;
use alkahest_data::tfx::{FeatureRendererSubscription, PipelineState, ShaderStage};
use glam::{Mat4, Vec4, vec4};

use super::Renderer;
use crate::{
    camera::Camera,
    cmd_event_span,
    gpu::command_list::CommandList,
    tfx::{
        externs::{self, GlobalLighting, ScreenArea, TextureView, UberDepth},
        scope::FrameScope,
        view::View,
    },
};

impl Renderer {
    pub fn submit_world(
        self: &Arc<Self>,
        cmd: &mut CommandList,
        view: &View,
        debug_pipeline: Option<DebugPipeline>,
    ) {
        cmd_event_span!(cmd, "submit_world");
        let _gpuspan = self.profiler.scope(cmd, "submit_world");

        *self.surfaces.write() = view.surfaces.clone();
        view.surfaces.resize_surfaces(view.resolution);

        self.active_feature_renderers
            .store(self.calculate_active_feature_renderers());

        let gpu = &self.gpu;

        self.prepare_externs(cmd, view);

        self.globals.scopes.view.bind(cmd).unwrap();

        let geo = if view.settings.multithreading {
            Some(self.submit_geometry_command_lists(cmd, view))
        } else {
            None
        };

        self.submit_gbuffer_generation(cmd, view, geo.as_ref());

        if matches!(
            debug_pipeline,
            Some(DebugPipeline::DeferredShading)
                | Some(DebugPipeline::DeferredShadingNoSun)
                | Some(DebugPipeline::LightDiffuse)
                | Some(DebugPipeline::LightSpecular)
        ) {
            self.submit_lighting(cmd, view, geo.as_ref());
        }

        self.submit_atmosphere(cmd, view);

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

        let output = view.surfaces.get(view.shading_result);
        self.profiler.scope(cmd, "debug_view").span(|| {
            cmd.clear_render_target_view(output.rtv.as_ref().unwrap(), &[0., 0., 0., 1.0]);
            output.bind_single(cmd);

            if let Some(debug_pipeline) = debug_pipeline {
                let p = &self.globals.pipelines;
                let technique = match debug_pipeline {
                    DebugPipeline::DeferredShading => &p.global_lighting_and_shading,
                    DebugPipeline::DeferredShadingNoSun => &p.deferred_shading_no_atm,
                    DebugPipeline::Albedo => &p.debug_source_color,
                    DebugPipeline::Smoothness => &p.debug_specular_smoothness,
                    DebugPipeline::Metalness => &p.debug_metalness,
                    DebugPipeline::AmbientOcclusion => &p.debug_ambient_occlusion,
                    DebugPipeline::Emission => &p.debug_emissive,
                    DebugPipeline::EmissionIntensity => &p.debug_emissive_intensity,
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

        // view.shading_result_read
        //     .lock()
        //     .update(cmd, view.surfaces.get(view.shading_result));

        if matches!(
            debug_pipeline,
            None | Some(DebugPipeline::DeferredShading) | Some(DebugPipeline::DeferredShadingNoSun)
        ) {
            self.submit_transparent(cmd, view, geo.as_ref());

            view.shading_result_read
                .lock()
                .update(cmd, view.surfaces.get(view.shading_result));

            self.submit_water(cmd, view);
            if debug_pipeline.is_some() {
                self.apply_volume_fog(cmd, view);
            }
            self.submit_bloom(cmd, view);

            // // Turn on bit 0x10 for all stencil buffer pixels
            // {
            //     cmd_event_span!(cmd, "set_stencil_bit_0x10");
            //     cmd.set_stencil_ref(0x10);
            //     cmd.state = PipelineState::new(Some(0), Some(77), Some(0), Some(0));
            //     cmd.flush_states();
            //     cmd.vertex_set_shader(Some(&self.common.blit_vs));
            //     cmd.pixel_set_shader(None);
            //     cmd.set_input_topology(PrimitiveType::TriangleStrip);
            //     cmd.draw(4, 0);
            // }

            // {
            //     cmd.set_stencil_ref(0);
            //     cmd.state = PipelineState::new(Some(0), Some(50), Some(0), Some(0));
            //     // Copies the sky lookup to the screen where depth is infinite, and masks out sky pixels in the stencil buffer
            //     self.execute_global_pipeline(cmd, &self.globals.pipelines.sky, "sky");
            // }

            cmd.set_stencil_ref(0);
            view.shading_result_read
                .lock()
                .update(cmd, view.surfaces.get(view.shading_result));
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
        } else {
            view.surfaces.get(view.shading_result).bind_single(cmd);
        }

        {
            profiling::scope!("prepare/submit immediate geometry");
            let _gpuspan = self.profiler.scope(cmd, "immediate_geometry");
            self.bind_surfaces(cmd, &[view.shading_result], Some(view.gbuffers.depth));
            cmd.state = PipelineState::new(Some(0), Some(2), Some(2), Some(0));
            cmd.flush_states();
            self.immediate.lock().prepare(gpu);
            self.immediate.lock().submit(cmd);
        }

        view.shading_result_read
            .lock()
            .update(cmd, view.surfaces.get(view.shading_result));

        {
            self.bind_surfaces(cmd, &[view.output], None);
            // Directly blit to output
            self.blit_srv(
                cmd,
                &view.shading_result_read.lock().srv,
                &view.surfaces.get(view.output).rtv,
                true,
                "final_blit_debug",
            );
        }

        // self.blit_srv(
        //     cmd,
        //     &view.shading_result_read.lock().srv,
        //     &view.surfaces.get(view.output).rtv,
        //     true,
        //     "final_blit",
        // );
    }

    fn prepare_externs(&self, cmd: &mut CommandList, view: &View) {
        let fb_res = view.surfaces.framebuffer_resolution();

        let misc = &self.frame_packet.read().misc;

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

        let global_tex = &self.globals.textures;

        *ext.frame = externs::Frame {
            game_time: misc.time,   //self.start_time.elapsed().as_secs_f32();
            render_time: misc.time, //self.start_time.elapsed().as_secs_f32();
            delta_game_time: misc.delta_time,
            unk10: misc.time_of_day,
            exposure_time: 0.016666668,
            exposure_scale: view.settings.exposure_scale,
            exposure_illum_relative: view.settings.exposure_illum_relative,
            specular_tint_lookup: global_tex.specular_tint_lookup.view.clone().into(),
            specular_lobe_lookup: global_tex.specular_lobe_lookup.view.clone().into(),
            specular_lobe_3d_lookup: global_tex.specular_lobe_3d_lookup.view.clone().into(),
            iridescence_lookup: global_tex.iridescence_lookup.view.clone().into(),
            ..*ext.frame.clone()
        };

        // TODO(cohae): Reconfirm the offset of iridescence lookup
        // let irr_lookup = &self.globals.textures.iridescence_lookup;
        // ext.frame.iridescence_lookup = irr_lookup.view.clone().into();

        let near = Camera::NEAR;
        let far = Camera::FAR;
        ext.deferred.depth_constants = vec4(
            1.0 / far,
            (far - near) / (far * near),
            0.00000000,
            0.00000000,
        );

        *ext.water_displacement = externs::WaterDisplacement {
            unk00: global_tex.water_displacement_unk00.view.clone().into(),
            unk08: global_tex.water_displacement_unk08.view.clone().into(),
            unk10: 0.045,
            unk14: 1.0, // value unknown, dont know where this is used
            unk18: 0.0,
            unk1c: 20.0,
            unk20: 600.0,
            unk24: 0.5,
            unk28: 2.0,
            unk2c: 0.0,
            unk30: 7.7,
        };

        // ext.deferred.gbuffer_resolution_scale_offset =
        //     vec4(fb_res.0 as f32, fb_res.1 as f32, 0.0, 0.0);
        ext.deferred.deferred_depth = view.gbuffers.depth_proxy.lock().srv.clone().into();
        ext.deferred.deferred_rt0 = view.gbuffers.albedo.into();
        ext.deferred.deferred_rt1 = view.gbuffers.normal.into();
        ext.deferred.deferred_rt2 = view.gbuffers.third.into();

        ext.deferred.light_diffuse = view.lighting.light_diffuse.into();
        ext.deferred.light_specular = view.lighting.light_specular.into();
        ext.deferred.light_specular_ibl = view.lighting.light_specular_ibl.into();

        ext.deferred.sky_hemisphere_mips = self.common.temporary_sky_hemisphere.view.clone().into();

        ext.decal.depth_read = view.gbuffers.depth_proxy.lock().srv.clone().into();
        ext.decal.normals_read = view.gbuffers.normal_read.into();
        ext.decal.depth_constants = ext.deferred.depth_constants;
        ext.decal.unk30 = vec4(fb_res.0 as f32, fb_res.1 as f32, 0.0, 0.0);

        ext.shadow_mask.unk00 = view.shadow_mask.into();
        // ext.shadow_mask.unk08 = view.lighting.ssao.into();
        ext.shadow_mask.unk10 = view.gbuffers.uber_depth_half.into();

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
            unkb0: vec4(0.01, 0.01, -0.5, -0.5),
            unkc0: vec4(0.02, -2.0, 0.0, 0.0),
            unkd0: vec4(0.00333, -2.33333, 0.00, 0.00),
            ..Default::default()
        };

        // ext.shadow_mask.unk00 = self.gpu.placeholder_white.view.clone().into();
        // ext.shadow_mask.unk08 = self.lighting.ssao.into();
        // ext.shadow_mask.unk10 = view.gbuffers.uber_depth_half.into();

        // if let Some(vao_srv) = view.surfaces.get(self.lighting.vertex_ao).srv.clone() {
        //     ext.cubemaps.vertex_ao = vao_srv.into();
        // }

        // ext.atmosphere.unk38 = self.common.temporary_depth_lookup.view.clone().into();
        // ext.atmosphere.unk88 = self.common.temporary_atmos.view.clone().into();

        *ext.atmosphere = externs::Atmosphere {
            time_of_day_normalized: misc.time_of_day,
            unk80: misc.atmosphere.atmosphere_lookup_vertical.clone().into(),
            unke0: view.atmosphere.sky_lookup_near.into(),
            unkf0: view.atmosphere.sky_lookup_far.into(),
            unk1ec: 150.0,
            unk1c4: 600.0,

            // From EDZ nighttime capture
            unk150: -0.97563,
            unk154: 0.00386,
            unk1b4: 0.94444,
            unk1b8: 1.00,
            unk1bc: 0.00427,
            // time_of_day_normalized: 0.05198,
            unk110: vec4(0.99605, 0.03099, -0.0832, 0.00),
            sky_lookup_resolution: view
                .surfaces()
                .get(view.atmosphere.sky_lookup_near)
                .resolution_with_recip(),
            unk1d0: vec4(0.00, 0.00, 0.00, 0.00),
            unk1e0: 0.00,
            unk1e4: 0.00386,
            unk1e8: 0.00427,

            ..Default::default()
        };

        // TODO(cohae): Most of these need to be verified, currently they are just shifted +0x70 from pre-BL
        ext.atmosphere.unk110 = ext.get_global_channel_by_name("sun_light_direction");

        // Sun
        ext.atmosphere.unk140 = ext.get_global_channel_by_id(0x56007c7);
        ext.atmosphere.unk150 = ext.get_global_channel_by_id(0x4aa1bef5).x;
        ext.atmosphere.unk154 = ext.get_global_channel_by_id(0x9859daf1).x;
        // ext.atmosphere.unk158 = ext.get_global_channel_by_id(0xc876108a).x;
        // ext.atmosphere.unk15c = ext.get_global_channel_by_id(0xe111d856).x;
        ext.atmosphere.unk160 = ext.get_global_channel_by_id(0xf853533c).x;

        // Fog
        ext.atmosphere.unk164 = ext.get_global_channel_by_id(0xed4bb08a).x;
        ext.atmosphere.unk168 = ext.get_global_channel_by_id(0x9e769ed2).x;
        ext.atmosphere.unk16c = ext.get_global_channel_by_id(0x49fbbce1).x;
        ext.atmosphere.unk170 = ext.get_global_channel_by_id(0x94d8ecdc).x;
        // ext.atmosphere.unk174 = ext.get_global_channel_by_id(0xbd2c7fe8).x;
        ext.atmosphere.unk180 = ext.get_global_channel_by_id(0x9ec7a5e8);
        ext.atmosphere.unk190 = ext.get_global_channel_by_id(0xb630810b).x;
        ext.atmosphere.unk194 = ext.get_global_channel_by_id(0x3eeacb23).x;
        ext.atmosphere.unk198 = ext.get_global_channel_by_id(0x7e92eb31).x;
        // ext.atmosphere.unk19c = ext.get_global_channel_by_id(0x9f4cb78f).x;

        // ext.atmosphere.unk1a0 = ext.get_global_channel_by_id(0x3e9cb6ed);
        // ext.atmosphere.unk1b0 = ext.get_global_channel_by_id(0x5fc9836).x;
        ext.atmosphere.unk1b4 = ext.get_global_channel_by_id(0xe283fbe0).x;
        ext.atmosphere.unk1b8 = ext.get_global_channel_by_id(0x5f3b8491).x;
        ext.atmosphere.unk1bc = ext.get_global_channel_by_id(0x79f2e305).x;
        ext.atmosphere.unk1c0 = ext.get_global_channel_by_id(0x62e4542e).x;
        ext.atmosphere.unk1c4 = ext.get_global_channel_by_id(0x949768cf).x;
        ext.atmosphere.unk1d0 = ext.get_global_channel_by_id(0xd9a2d8a3);
        ext.atmosphere.unk1e0 = ext.get_global_channel_by_id(0xd8281393).x;
        ext.atmosphere.unk1e4 = ext.get_global_channel_by_id(0x4da73ca7).x;
        ext.atmosphere.unk1e8 = ext.get_global_channel_by_id(0xe685c537).x;
        ext.atmosphere.unk1ec = ext.get_global_channel_by_id(0xe4a1bf60).x;
        // ext.atmosphere.unk1f0 = ext.get_global_channel_by_id(0x63d92f7).x;
        // ext.atmosphere.unk1f4 = ext.get_global_channel_by_id(0x49864a42).x;

        ext.screen_area = ScreenArea {
            unk00: view.shading_result_read.lock().srv.clone().into(),
            unk30: TextureView::None, // health overlay
            unk38: self.common.default_lut.view.clone().into(), // LUT
            unk40: if view.settings.bloom {
                view.bloom.bloom_final.into()
            } else {
                self.common.temporary_bloom.view.clone().into()
            }, // bloom
            unk48: view.lighting.distortion.into(), // distortion
            unk58: self.common.temporary_vignette.view.clone().into(), // vignette
            unk7c: 0.9968,

            // unk80: 0.9968, // Skydock IV
            unk80: 0.1, // Orbit

            unk90: vec4(32.0, 1024.0, 0.0, 0.0),
            unka0: vec4(0.03125, -5.0, 14.0, 2.5),
            unkb0: 0.5,
            unkb4: 2.0,
            unke0: vec4(0.25, -0.225, 0.40, 0.96),
            unkf0: vec4(0.13281, 0.23611, 0.00, 0.00), // distortion related
            // unkf0: Vec4::ZERO,
            unk140: 0.05,
            unk150: vec4(0.3, 0.5, 0.0, 0.02),
            unk160: vec4(0.3, 0.5, 0.0, 0.5),
            ..Default::default()
        }
        .into();

        let depth_res = view.surfaces.get(view.gbuffers.depth).resolution();
        ext.uber_depth = UberDepth {
            original_depth: view.gbuffers.depth_proxy.lock().srv.clone().into(),
            unk30: view.gbuffers.uber_depth_half.into(),
            unk40: view.gbuffers.uber_depth_quarter.into(),
            unk50: ext.deferred.depth_constants,
            unk70: vec4(0.0, 0.0, depth_res.0 as f32, depth_res.1 as f32),
            ..Default::default()
        }
        .into();

        ext.cubemaps.unk00 = view.lighting.vertex_ao.into();

        *ext.transparent = externs::Transparent {
            unk00: view.atmosphere.sky_lookup_near.into(),
            unk10: view.atmosphere.sky_lookup_far.into(),
            // unk00: todo!(), // t11, Atmosphere (near?)
            // unk08: todo!(), // t12, Atmosphere (3x2)
            // unk10: todo!(), // t13, Atmosphere (far?)
            // unk18: todo!(), // t14, 3d lightprobe
            unk20: self.common.temporary_depth_angle_lookup.view.clone().into(), // t15
            // unk28: todo!(), // t16, 3d lightprobe
            // unk30: todo!(), // t17, 3d lightprobe
            // unk38: todo!(), // t18, 3d lightprobe
            // unk40: todo!(), // t19, 3d lightprobe
            // unk48: todo!(), // t20
            // unk50: todo!(), // t21
            // unk58: todo!(), // t22
            // unk60: todo!(), // t23
            unk70: vec4(0.22882, 0.00, 1.00, 45.00),
            unk80: vec4(0.00, 0.00, 1.17485, 2.86546),
            unk90: vec4(0.00, 0.00, 2.10913, 5.14044),
            unka0: vec4(0.00, 0.00, 3.46762, 8.41667),
            unkb0: vec4(0.00, 0.00, 0.00, 0.00),
            ..Default::default()
        };

        self.globals
            .scopes
            .transparent_advanced
            .write_initial_constants(
                cmd,
                &[
                    vec4(0.00227, 0.00896, 0.32782, 0.6419),
                    vec4(0.0026, 4.86115, 0.00198, 0.00002),
                    vec4(0.9158, 233.93063, 0.51102, 0.08905),
                    vec4(147.09909, 0.55492, 0.52397, 0.00),
                    vec4(0.00, 0.64794, 0.14063, 0.01563),
                    vec4(0.58584, 0.58584, 0.58584, 0.58584),
                    vec4(1.38137, 2.08133, 0.85451, 0.4165),
                    vec4(0.90933, 0.90933, 0.90933, 0.90933),
                    vec4(132.92885, 66.40444, 56.85342, 0.00),
                    vec4(132.92885, 66.40444, 1000.00, 0.0001),
                    vec4(131.92885, 65.40444, 55.85342, 0.67843),
                    vec4(131.92885, 65.40444, 999.00, 5.50),
                    vec4(0.00, 0.50, 25.57599, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.025, 10000.00, -9999.00, 1.00),
                    vec4(1.00, 1.00, 1.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(10.92799, 7.10136, 6.25467, 0.00),
                    vec4(0.00376, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00753, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.01759, 0.00),
                    vec4(-1.13485, 6.87303, -0.33715, 1.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(0.00, 0.00, 0.00, 0.00),
                    vec4(1.00, 0.00, 0.00, 0.00),
                ],
            )
            .expect("Failed to write transparent_advanced initial constants");

        let _ = self.globals.scopes.frame.write_initial_constants(
            cmd,
            FrameScope {
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

                random_seed_scales: vec4(
                    (misc.time * 60.0 + 33.75) * 1.258699,
                    (misc.time * 60.0 + 60.0) * 0.9583125,
                    (misc.time * 60.0 + 60.0) * 8.789123,
                    (misc.time * 60.0 + 33.75) * 2.311535,
                ),
                unk3: vec4(0.5, 0.5, 0.0, 0.0),
                unk4: vec4(1.0, 1.0, 0.0, 1.0),
                unk5: vec4(0.00, -f32::NAN, 512.00, 0.00),
                unk6: Vec4::ONE,
            }
            .to_array()
            .as_ref(),
        );

        self.globals.scopes.frame.bind(cmd).unwrap();
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DebugPipeline {
    DeferredShading,
    DeferredShadingNoSun,

    Albedo,
    Smoothness,
    Metalness,
    AmbientOcclusion,
    Emission,
    EmissionIntensity,
    Transmission,
    Overcoat,

    DepthEdges,
    WorldNormal,

    LightDiffuse,
    LightSpecular,
}
