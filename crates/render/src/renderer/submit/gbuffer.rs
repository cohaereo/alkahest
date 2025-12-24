use alkahest_core::convar::ConVars;
use alkahest_data::tfx::{
    FeatureRendererSubscription, PipelineState, RenderStage, TfxFeatureRenderer,
};

use super::Renderer;
use crate::{
    cmd_event_span,
    gpu::command_list::{CommandList, DepthMode},
    tfx::view::View,
};

impl Renderer {
    pub(super) fn submit_gbuffer_generation(&self, cmd: &mut CommandList, view: &View) {
        profiling::scope!("submit_gbuffer_generation");
        let _gpuscope = self.profiler.scope(cmd, "submit_gbuffer");

        view.gbuffers.clear(cmd, &view.surfaces);
        view.gbuffers.bind(cmd, self);

        {
            cmd_event_span!(cmd, "generate_gbuffer");
            let _gpuscope = self.profiler.scope(cmd, "generate_buffer");

            cmd.state = PipelineState::new(Some(0), Some(2), Some(2), Some(0));

            self.submit_stage_multi(cmd, RenderStage::GenerateGbuffer, 16);
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
            self.submit_stage(
                cmd,
                RenderStage::Decals,
                FeatureRendererSubscription::all_but(TfxFeatureRenderer::DynamicDecals),
            );

            // TODO(cohae): We should only reverse the depth mode for decals that we are inside of
            cmd.state_override = PipelineState::new(None, None, Some(1), None);
            cmd.set_depth_mode(DepthMode::Forward);
            self.submit_stage(
                cmd,
                RenderStage::Decals,
                FeatureRendererSubscription::DYNAMIC_DECALS,
            );
            cmd.set_depth_mode(DepthMode::Reverse);
            cmd.state_override.reset();
        }

        view.gbuffers
            .third_proxy
            .lock()
            .update(cmd, view.surfaces.get(view.gbuffers.third));

        // TODO(cohae): Can we reduce boilerplate for these kinds of pipelines?
        if ConVars::get_flag("render.vertex_ao_workaround") {
            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            cmd.flush_states();
            let third_surf = view.surfaces.get(view.gbuffers.third);
            let vao_surf = view.surfaces.get(view.lighting.vertex_ao);
            cmd.output_merger_set_render_targets(
                &[None, None, third_surf.rtv.as_ref(), vao_surf.rtv.as_ref()],
                None,
            );
            cmd.vertex_set_shader(Some(&self.clear_ao_vs));
            if view.settings.vertex_ao {
                cmd.pixel_set_shader(Some(&self.clear_ao_ps));
            } else {
                cmd.pixel_set_shader(Some(&self.clear_ao_all_ps));
            }
            cmd.set_input_topology(alkahest_data::tfx::PrimitiveType::TriangleStrip);
            cmd.pixel_set_shader_resources(0, &[Some(&view.gbuffers.third_proxy.lock().srv)]);
            cmd.draw(4, 0);
        }

        // {
        //     cmd.state = PipelineState::new(Some(0), Some(2), Some(0), Some(0));
        //     let depth_half_surf = view.surfaces.get(view.gbuffers.depth_half);
        //     depth_half_surf.clear_depth(cmd, 0.0, 0);
        //     depth_half_surf.bind_single(cmd);
        //     let depth_full_surf = view.surfaces.get(view.gbuffers.depth);

        //     {
        //         let hdao = &mut self.externs.get_mut().hdao;
        //         hdao.unk60_source = view.gbuffers.depth_proxy.lock().srv.clone().into();
        //         hdao.unk70_dest_res = depth_half_surf.resolution_with_recip();
        //         hdao.unk80_source_res = depth_full_surf.resolution_with_recip();
        //     }

        //     self.execute_global_pipeline(
        //         cmd,
        //         &self.globals.pipelines.downsample_depth_buffer,
        //         "downsample_depth_buffer",
        //     );
        // }

        // self.submit_uber_depth_generation(cmd);
    }

    // fn submit_uber_depth_generation(&self, cmd: &mut CommandList) {
    //     cmd_event_span!(cmd, "submit_uber_depth_generation");

    //     {
    //         cmd_event_span!(cmd, "[uber_depth_default]");

    //         self.globals.pipelines.uber_depth_default.bind(cmd).unwrap();
    //         let (width, height) = view.surfaces.get(view.gbuffers.depth).resolution();
    //         cmd.dispatch(width.div_ceil(16), height.div_ceil(16), 1);
    //         cmd.compute_set_unordered_access_views(0, &[None, None, None, None], None);
    //     }

    //     cmd_event_span!(cmd, "[downsample_max_min_avg_no_swizzle]");
    //     self.externs.get_mut().downsample_texture_generic = DownsampleTextureGeneric {
    //         source: view.gbuffers.uber_depth_quarter.into(),
    //         resolution_dest: self
    //             .surfaces
    //             .get(view.gbuffers.uber_depth_eighth)
    //             .resolution_with_recip(),
    //         resolution_source: self
    //             .surfaces
    //             .get(view.gbuffers.uber_depth_quarter)
    //             .resolution_with_recip(),
    //     };
    //     self.bind_surfaces(cmd, &[view.gbuffers.uber_depth_eighth], None);
    //     cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
    //     self.execute_global_pipeline(
    //         cmd,
    //         &self.globals.pipelines.downsample_max_min_avg_no_swizzle,
    //         "downsample_max_min_avg_no_swizzle",
    //     );
    // }
}
