use alkahest_data::tfx::{FeatureRendererSubscription, PipelineState, RenderStage};
use glam::Vec4;

use super::Renderer;
use crate::{cmd_event_span, gpu::command_list::CommandList};

impl Renderer {
    pub(super) fn submit_water(&self, cmd: &mut CommandList) {
        cmd_event_span!(cmd, "water_reflection");
        {
            let ext = &mut self.externs.get_mut();
            ext.deferred.deferred_depth = self.gbuffers.depth_proxy.lock().srv.clone().into();
            ext.view
                .derive_matrices(self.surfaces.get(self.water.water_uv).resolution());
        }
        self.globals.scopes.view.bind(cmd).unwrap();

        self.clear_surface(cmd, self.water.water_uv, [0., 0., 0., 0.]);
        self.clear_surface_depth(cmd, self.water.water_depth, 0.0, 0);
        cmd.state = PipelineState::new(Some(1), Some(1), Some(2), Some(1));
        self.bind_surfaces(cmd, &[self.water.water_uv], Some(self.water.water_depth));
        self.submit_stage(
            cmd,
            RenderStage::WaterReflection,
            FeatureRendererSubscription::all(),
        );

        {
            let ext = &mut self.externs.get_mut();
            ext.postprocess.unk00 = self.water.water_uv.into();
            ext.postprocess.unkb0 = Vec4::new(5.0, 1.0, 1.0, 1.0);
            ext.postprocess.res_for_unk00 = self
                .surfaces
                .get(self.water.water_uv)
                .resolution_with_recip();
            ext.postprocess.output_res = self
                .surfaces
                .get(self.water.water_uv_healed)
                .resolution_with_recip();

            ext.view
                .derive_matrices(self.surfaces.framebuffer_resolution());
        }
        self.globals.scopes.view.bind(cmd).unwrap();
        cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
        self.bind_surfaces(cmd, &[self.water.water_uv_healed], None);
        self.execute_global_pipeline(
            cmd,
            &self.globals.pipelines.water_reflection_uv_healing,
            "water_reflection_uv_healing",
        );

        {
            let ext = &mut self.externs.get_mut();
            ext.postprocess.unk00 = self.shading_result_read.lock().srv.clone().into();
            ext.postprocess.unk08 = self.water.water_uv_healed.into();
            ext.postprocess.unkb0 = Vec4::new(1.0, 1.0, 1.0, 1.0);
            ext.postprocess.res_for_unk00 = self
                .surfaces
                .get(self.shading_result)
                .resolution_with_recip();
            ext.postprocess.output_res = self
                .surfaces
                .get(self.water.water_reflection)
                .resolution_with_recip();
            // ext.view
            //     .derive_matrices(self.surfaces.get(self.water.water_reflection).resolution());
        }
        self.globals.scopes.view.bind(cmd).unwrap();

        cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
        self.bind_surfaces(cmd, &[self.water.water_reflection], None);
        self.execute_global_pipeline(
            cmd,
            &self.globals.pipelines.water_reflection_resolve,
            "water_reflection_resolve",
        );

        {
            let ext = &mut self.externs.get_mut();
            ext.postprocess.unk00 = self.water.water_reflection.into();
            ext.postprocess.unk08 = self.water.water_reflection_healed.into();
            ext.postprocess.unkb0 = Vec4::new(5.0, 1.0, 1.0, 1.0);
            ext.postprocess.res_for_unk00 = self
                .surfaces
                .get(self.water.water_reflection)
                .resolution_with_recip();
            ext.postprocess.output_res = self
                .surfaces
                .get(self.water.water_reflection_healed)
                .resolution_with_recip();
        }

        cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
        self.bind_surfaces(cmd, &[self.water.water_reflection_healed], None);
        self.execute_global_pipeline(
            cmd,
            &self.globals.pipelines.water_reflection_healing,
            "water_reflection_healing",
        );

        {
            let ext = &mut self.externs.get_mut();
            ext.water.unk00 = self.shading_result_read.lock().srv.clone().into();
            ext.water.unk08 = self.water.water_uv.into();
            ext.water.unk30 = self.water.water_reflection_healed.into();
        }

        self.submit_water_planes(cmd);
    }

    fn submit_water_planes(&self, cmd: &mut CommandList) {
        {
            {
                let ext = &mut self.externs.get_mut();
                ext.deferred.deferred_depth = self.gbuffers.depth_proxy.lock().srv.clone().into();
                ext.view
                    .derive_matrices(self.surfaces.get(self.shading_result).resolution());
            }
            self.globals.scopes.view.bind(cmd).unwrap();
            self.globals.scopes.transparent.bind(cmd).unwrap();
            self.globals.scopes.transparent_advanced.bind(cmd).unwrap();
        }

        {
            cmd_event_span!(cmd, "water");

            self.bind_surfaces(cmd, &[self.shading_result], Some(self.gbuffers.depth));
            cmd.state = PipelineState::new(Some(8), Some(15), Some(2), Some(1));
            self.submit_stage(
                cmd,
                RenderStage::Transparents,
                FeatureRendererSubscription::WATER,
            );
        }
    }
}
