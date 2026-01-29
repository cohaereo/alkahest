use std::sync::Arc;

use alkahest_data::tfx::{
    FeatureRendererSubscription, PipelineState, RenderStage, TfxFeatureRenderer,
};

use super::Renderer;
use crate::{
    cmd_event_span,
    gpu::command_list::CommandList,
    renderer::submit::geometry::GeometryCommandLists,
    tfx::view::{MainView, View},
};

impl Renderer {
    pub(super) fn submit_transparent(
        self: &Arc<Self>,
        cmd: &mut CommandList,
        view: &MainView,
        _geo: Option<&GeometryCommandLists>,
    ) {
        {
            {
                let ext = &mut self.externs.get_mut();
                ext.deferred.deferred_depth = view.gbuffers.depth_proxy.lock().srv.clone().into();
                ext.view
                    .derive_matrices(self.surfaces().get(view.shading_result).resolution());
            }
            self.globals.scopes.view.bind(cmd).unwrap();
            self.globals.scopes.transparent.bind(cmd).unwrap();
            self.globals.scopes.transparent_advanced.bind(cmd).unwrap();

            cmd_event_span!(cmd, "decals_additive");
            let _gpuscope = self.profiler.scope(cmd, "decals_additive");
            self.bind_surfaces(cmd, &[view.shading_result], Some(view.gbuffers.depth));

            cmd.state = PipelineState::new(Some(8), Some(15), Some(2), Some(1));
            cmd.flush_states();
            self.submit_stage_parallel_apply(
                cmd,
                View::MAIN,
                RenderStage::DecalsAdditive,
                FeatureRendererSubscription::all(),
            );
        }
        {
            cmd_event_span!(cmd, "transparents");
            let _gpuscope = self.profiler.scope(cmd, "transparents");

            cmd.state = PipelineState::new(Some(8), Some(15), Some(2), Some(1));

            if self.settings().multithreading {
                self.submit_stage_parallel_linear(
                    cmd,
                    View::MAIN,
                    RenderStage::Transparents,
                    FeatureRendererSubscription::all_but(TfxFeatureRenderer::Water),
                );
            } else {
                self.submit_stage(
                    cmd,
                    View::MAIN,
                    RenderStage::Transparents,
                    FeatureRendererSubscription::all_but(TfxFeatureRenderer::Water),
                );
            }
        }

        {
            cmd_event_span!(cmd, "distortion");
            let _gpuscope = self.profiler.scope(cmd, "distortion");

            {
                let distortion = view.surfaces.get(view.lighting.distortion);
                let externs = &mut self.externs.get_mut();
                externs.view.derive_matrices(distortion.resolution());
                externs.deferred.deferred_depth = view.gbuffers.uber_depth_half.into();
            }
            self.globals.scopes.view.bind(cmd).unwrap();

            self.clear_surface(cmd, view.lighting.distortion, [0., 0., 0., 0.]);
            self.bind_surfaces(
                cmd,
                &[view.lighting.distortion],
                Some(view.gbuffers.depth_half),
            );

            cmd.state = PipelineState::new(Some(8), Some(15), Some(2), Some(1));
            cmd.flush_states();
            self.submit_stage(
                cmd,
                View::MAIN,
                RenderStage::Distortion,
                FeatureRendererSubscription::all(),
            );
        }

        // Rebind full resolution depth buffer
        {
            let output = view.surfaces.get(view.output);
            let externs = &mut self.externs.get_mut();
            externs.view.derive_matrices(output.resolution());
            externs.deferred.deferred_depth = view.gbuffers.depth.into();
        }
        self.globals.scopes.view.bind(cmd).unwrap();
    }
}
