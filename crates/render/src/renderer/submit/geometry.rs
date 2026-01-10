use std::sync::Arc;

use alkahest_core::job::potassium::JobHandle;
use alkahest_data::tfx::{FeatureRendererSubscription, PipelineState, RenderStage};

use crate::{
    Renderer, cmd_event_span, gpu::command_list::CommandList, tfx::view::View,
    util::threading::CommandListSetId,
};

pub struct GeometryCommandLists {
    pub generate_gbuffer: (JobHandle, CommandListSetId),
    pub decals: (JobHandle, CommandListSetId),
    pub lighting: (JobHandle, CommandListSetId),
    // pub transparent: (JobHandle, CommandListSetId),
    // pub volumetrics: (JobHandle, CommandListSetId),
}

impl Renderer {
    pub fn submit_geometry_command_lists(
        self: &Arc<Self>,
        cmd: &mut CommandList,
        view: &View,
    ) -> GeometryCommandLists {
        cmd_event_span!(cmd, "geometry_early");
        let _gpuscope = self.profiler.scope(cmd, "geometry_early");

        let generate_gbuffer = {
            view.gbuffers.bind(cmd, self);
            cmd.state = PipelineState::new(Some(0), Some(2), Some(2), Some(0));

            self.submit_stage_parallel(
                cmd,
                RenderStage::GenerateGbuffer,
                FeatureRendererSubscription::all(),
            )
        };

        let decals = {
            view.gbuffers.bind(cmd, self);
            cmd.state = PipelineState::new(Some(8), Some(15), Some(2), Some(0));

            self.submit_stage_parallel(cmd, RenderStage::Decals, FeatureRendererSubscription::all())
        };

        let lighting = {
            view.lighting.bind_diffuse_specular(cmd, &view.surfaces);
            cmd.state = PipelineState::new(Some(8), None, Some(2), Some(2));
            cmd.flush_states();
            {
                self.submit_stage_parallel(
                    cmd,
                    RenderStage::LightingApply,
                    FeatureRendererSubscription::all(),
                )
            }
        };

        // let volumetrics = {
        //     {
        //         let ext = self.externs.get_mut();
        //         // ext.deferred.depth_constants = Vec4::new(0.0, 1. / 0.01, 0.0, 0.0);
        //         ext.deferred.deferred_depth = view.gbuffers.uber_depth_eighth.into();
        //         let volumetrics = view.surfaces.get(view.lighting.volumetrics_rt0);
        //         ext.view.derive_matrices(volumetrics.resolution());
        //     }
        //     self.globals.scopes.view.bind(cmd).unwrap();
        //     self.globals.scopes.transparent.bind(cmd).unwrap();

        //     view.lighting.bind_volumetrics(self, cmd);
        //     cmd.state = PipelineState::new(Some(8), None, Some(2), Some(2));
        //     {
        //         self.submit_stage_parallel_linear(
        //             cmd,
        //             RenderStage::Volumetrics,
        //             FeatureRendererSubscription::all(),
        //         )
        //     }
        // };

        // let transparent = {
        //     self.bind_surfaces(cmd, &[view.shading_result], Some(view.gbuffers.depth));
        //     cmd.state = PipelineState::new(Some(8), Some(15), Some(2), Some(1));
        //     self.submit_stage_parallel_linear(
        //         cmd,
        //         RenderStage::Transparents,
        //         FeatureRendererSubscription::all_but(TfxFeatureRenderer::Water)
        //             .without(TfxFeatureRenderer::SkyTransparent)
        //             .without(TfxFeatureRenderer::RigidObject),
        //     )
        // };

        GeometryCommandLists {
            generate_gbuffer,
            decals,
            lighting,
            // transparent,
            // volumetrics,
        }
    }
}
