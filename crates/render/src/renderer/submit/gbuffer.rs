use std::sync::Arc;

use alkahest_data::tfx::{
    FeatureRendererSubscription, PipelineState, PrimitiveType, RenderStage, ShaderStage,
    TfxFeatureRenderer,
};
use glam::UVec2;

use super::Renderer;
use crate::{
    cmd_event_span,
    gpu::command_list::CommandList,
    renderer::{hzb::Hzb, submit::geometry::GeometryCommandLists},
    tfx::{externs::DownsampleTextureGeneric, view::View},
};

impl Renderer {
    pub(super) fn submit_gbuffer_generation(
        self: &Arc<Self>,
        cmd: &mut CommandList,
        view: &View,
        geo: Option<&GeometryCommandLists>,
    ) {
        profiling::scope!("submit_gbuffer_generation");
        let _gpuscope = self.profiler.scope(cmd, "submit_gbuffer");

        view.gbuffers.clear(cmd, &view.surfaces);
        view.gbuffers.bind(cmd, self);

        {
            cmd_event_span!(cmd, "generate_gbuffer");
            let _gpuscope = self.profiler.scope(cmd, "generate_buffer");

            cmd.state = PipelineState::new(Some(0), Some(2), Some(2), Some(0));

            if let Some(geo) = geo {
                let (sync_job, set) = &geo.generate_gbuffer;
                sync_job.wait();
                self.cmd_pool.finish(cmd, *set);
            } else {
                // cmd.state = PipelineState::new(Some(0), Some(2), Some(1), Some(0));
                // view.gbuffers.bind_depth_only(cmd, self);
                // self.submit_stage(
                //     cmd,
                //     RenderStage::DepthPrepass,
                //     FeatureRendererSubscription::all(),
                // );
                // cmd.state = PipelineState::new(Some(0), Some(2), Some(2), Some(0));
                // view.gbuffers.bind(cmd, self);
                self.submit_stage(
                    cmd,
                    RenderStage::GenerateGbuffer,
                    FeatureRendererSubscription::all(),
                );
            }
        }

        {
            cmd_event_span!(cmd, "decals");
            let _gpuscope = self.profiler.scope(cmd, "decals");

            view.gbuffers
                .depth_proxy
                .lock()
                .update(cmd, view.surfaces.get(view.gbuffers.depth));

            view.surfaces
                .copy(cmd, view.gbuffers.normal, view.gbuffers.normal_read);
            cmd.state = PipelineState::new(Some(8), Some(15), Some(2), Some(0));

            if let Some(geo) = geo {
                let (sync_job, set) = &geo.decals;
                sync_job.wait();
                self.cmd_pool.finish(cmd, *set);
            } else {
                self.submit_stage(
                    cmd,
                    RenderStage::Decals,
                    FeatureRendererSubscription::all_but(TfxFeatureRenderer::DynamicDecals)
                        .without(TfxFeatureRenderer::RoadDecals),
                );
            }

            self.submit_stage_parallel_linear(
                cmd,
                RenderStage::Decals,
                FeatureRendererSubscription::DYNAMIC_DECALS
                    | FeatureRendererSubscription::ROAD_DECALS,
            );
        }

        view.gbuffers
            .albedo_proxy
            .lock()
            .update(cmd, view.surfaces.get(view.gbuffers.albedo));
        view.gbuffers
            .third_proxy
            .lock()
            .update(cmd, view.surfaces.get(view.gbuffers.third));

        // TODO(cohae): Can we reduce boilerplate for these kinds of pipelines?
        if true {
            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            cmd.flush_states();
            let albedo_surf = view.surfaces.get(view.gbuffers.albedo);
            let third_surf = view.surfaces.get(view.gbuffers.third);
            let vao_surf = view.surfaces.get(view.lighting.vertex_ao);
            cmd.output_merger_set_render_targets(
                &[
                    albedo_surf.rtv.as_ref(),
                    None,
                    third_surf.rtv.as_ref(),
                    vao_surf.rtv.as_ref(),
                ],
                None,
            );
            cmd.vertex_set_shader(Some(&self.clear_ao_vs));
            if view.settings.vertex_ao {
                cmd.pixel_set_shader(Some(&self.clear_ao_ps));
            } else {
                cmd.pixel_set_shader(Some(&self.clear_ao_all_ps));
            }
            cmd.set_input_topology(alkahest_data::tfx::PrimitiveType::TriangleStrip);
            cmd.pixel_set_shader_resources(
                0,
                &[
                    Some(&view.gbuffers.albedo_proxy.lock().srv),
                    Some(&view.gbuffers.third_proxy.lock().srv),
                ],
            );
            cmd.draw(4, 0);
        }

        {
            cmd.state = PipelineState::new(Some(0), Some(2), Some(0), Some(0));
            let depth_half_surf = view.surfaces.get(view.gbuffers.depth_half);
            depth_half_surf.clear_depth(cmd, 0.0, 0);
            depth_half_surf.bind_single(cmd);
            let depth_full_surf = view.surfaces.get(view.gbuffers.depth);

            {
                let hdao = &mut self.externs.get_mut().hdao;
                hdao.unk60_source = view.gbuffers.depth_proxy.lock().srv.clone().into();
                hdao.unk70_dest_res = depth_half_surf.resolution_with_recip();
                hdao.unk80_source_res = depth_full_surf.resolution_with_recip();
            }

            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.downsample_depth_buffer,
                "downsample_depth_buffer",
            );
        }

        self.submit_uber_depth_generation(cmd, view);
        self.generate_hzb_chain(cmd, view);
    }

    fn submit_uber_depth_generation(&self, cmd: &mut CommandList, view: &View) {
        cmd_event_span!(cmd, "submit_uber_depth_generation");
        let _gpuscope = self.profiler.scope(cmd, "uber_depth_generation");

        {
            cmd_event_span!(cmd, "[uber_depth_default]");

            self.globals.pipelines.uber_depth_default.bind(cmd).unwrap();
            let (width, height) = view.surfaces.get(view.gbuffers.depth).resolution();
            cmd.dispatch(width.div_ceil(16), height.div_ceil(16), 1);
            cmd.compute_set_unordered_access_views(0, &[None, None, None, None], None);
        }

        cmd_event_span!(cmd, "[downsample_max_min_avg_no_swizzle]");
        self.externs.get_mut().downsample_texture_generic = DownsampleTextureGeneric {
            source: view.gbuffers.uber_depth_quarter.into(),
            resolution_dest: view
                .surfaces
                .get(view.gbuffers.uber_depth_eighth)
                .resolution_with_recip(),
            resolution_source: view
                .surfaces
                .get(view.gbuffers.uber_depth_quarter)
                .resolution_with_recip(),
        }
        .into();
        self.bind_surfaces(cmd, &[view.gbuffers.uber_depth_eighth], None);
        cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
        self.execute_global_pipeline(
            cmd,
            &self.globals.pipelines.downsample_max_min_avg_no_swizzle,
            "downsample_max_min_avg_no_swizzle",
        );
    }

    fn generate_hzb_chain(&self, cmd: &mut CommandList, view: &View) {
        cmd_event_span!(cmd, "generate_hzb_chain");
        let _gpuscope = self.profiler.scope(cmd, "generate_hzb_chain");

        let hzb_chain = view.surfaces().get(view.gbuffers.hzb_depth_chain);

        let depth = view.gbuffers.depth_proxy.lock();
        // Copy main depth to mip 0
        {
            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            cmd.flush_states();
            cmd.rasterizer_set_viewports(&[d3d11::Viewport::builder()
                .width(hzb_chain.resolution().0 as f32)
                .height(hzb_chain.resolution().1 as f32)
                .build()]);
            cmd.vertex_set_shader(Some(&self.common.blit_vs));
            cmd.pixel_set_shader(Some(&self.common.blit_ps_linear));
            cmd.set_input_topology(PrimitiveType::TriangleStrip);
            hzb_chain.bind_single(cmd);
            cmd.pixel_set_shader_resources(0, std::slice::from_ref(&Some(&depth.srv)));
            cmd.draw(4, 0);
        }

        cmd.compute_set_samplers(1, &[Some(&self.common.sampler_point)]);
        cmd.compute_set_shader(&self.hzb_downsample_cs);
        let (mut width, mut height) = hzb_chain.resolution();
        for mip in 1..(hzb_chain.mip_count - 1) {
            let current_width = (width >> 1).max(1);
            let current_height = (height >> 1).max(1);

            let srv = if mip == 1 {
                Some(depth.srv.clone())
            } else {
                hzb_chain.srv(mip as usize - 1).cloned()
            };

            cmd.compute_set_unordered_access_views(0, &[hzb_chain.uav(mip as usize)], None);
            cmd.compute_set_shader_resources(0, &[srv.as_ref()]);

            _ = self.hzb_downsample_params.write(
                cmd,
                &HzbDownsampleParams {
                    prev_size: UVec2::new(width, height),
                    current_size: UVec2::new(current_width, current_height),
                },
            );
            self.hzb_downsample_params
                .bind(cmd, ShaderStage::Compute, 0);

            const GROUP_SIZE: u32 = 8;
            let x = current_width.div_ceil(GROUP_SIZE);
            let y = current_height.div_ceil(GROUP_SIZE);
            cmd.dispatch(x, y, 1);

            width = current_width;
            height = current_height;
        }

        {
            let _gpuscope = self.profiler.scope(cmd, "copy_hzb_to_cpu");

            let num_mips = hzb_chain.mip_count;

            let mip_range = num_mips.saturating_sub(Hzb::MAX_MIP_COUNT + 1)..num_mips;
            view.gbuffers
                .hzb_depth_chain_cpu
                .lock()
                .update_mips(cmd, hzb_chain, mip_range);
        }
    }
}

#[repr(C)]
pub struct HzbDownsampleParams {
    prev_size: UVec2,
    current_size: UVec2,
}
