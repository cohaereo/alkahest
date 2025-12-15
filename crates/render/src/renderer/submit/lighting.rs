use alkahest_data::tfx::{PipelineState, RenderStage};

use super::Renderer;
use crate::{cmd_event_span, gpu::command_list::CommandList, tfx::view::View};

impl Renderer {
    pub(super) fn submit_lighting(&self, cmd: &mut CommandList, view: &View) {
        // self.compute_ssao(cmd);
        profiling::scope!("submit_lighting");
        let _gpuspan = self.profiler.scope(cmd, "submit_lighting");
        {
            let shading_result = view.surfaces.get(view.shading_result);
            let ext = self.externs.get_mut();
            ext.deferred.deferred_depth = view.gbuffers.depth_proxy.lock().srv.clone().into();
            ext.view.derive_matrices(shading_result.resolution());
        }
        self.globals.scopes.view.bind(cmd).unwrap();

        view.lighting.clear(cmd, &view.surfaces);
        view.lighting.bind_ibl_vertex_ao(cmd, &view.surfaces);

        cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
        // if ConVars::get_flag("render.sky") {
        //     self.execute_global_pipeline(
        //         cmd,
        //         &self.globals.pipelines.cubemap_apply_sky_copy_ao,
        //         "cubemap_apply_sky_copy_ao",
        //     );
        // } else {
        //     cmd.clear_render_target_view(
        //         view.surfaces
        //             .get(view.lighting.vertex_ao)
        //             .rtv
        //             .as_ref()
        //             .unwrap(),
        //         &[0.5, 0.5, 0.5, 1.],
        //     );
        // }

        cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
        // view.lighting.bind_diffuse_specular(cmd, &view.surfaces);
        // self.execute_global_pipeline(
        //     cmd,
        //     &self.globals.pipelines.global_lighting,
        //     "global_lighting",
        // );

        view.lighting.bind_diffuse_ibl(cmd, &view.surfaces);
        cmd.state = PipelineState::new(Some(23), Some(1), Some(3), Some(1));
        cmd.flush_states();
        {
            cmd_event_span!(cmd, "cubemaps");
            let _gpuspan = self.profiler.scope(cmd, "cubemaps");
            // self.submit_stage(
            //     cmd,
            //     RenderStage::Cubemaps,
            //     FeatureRendererSubscription::all(),
            // );
        }
        view.lighting.bind_diffuse_specular(cmd, &view.surfaces);
        cmd.state = PipelineState::new(Some(8), None, Some(2), Some(2));
        {
            cmd_event_span!(cmd, "local_lights");
            let _gpuspan = self.profiler.scope(cmd, "local_lights");
            self.submit_stage_multi(cmd, RenderStage::LightingApply, 16);
        }

        // self.submit_volumetrics(cmd);
    }

    // pub(super) fn submit_volumetrics(&self, cmd: &mut CommandList) {
    //     profiling::scope!("submit_volumetrics");

    //     // cmd.pixel_set_shader_resources(
    //     //     10,
    //     //     &[self
    //     //         .surfaces
    //     //         .get(self.gbuffers.uber_depth_eigth)
    //     //         .srv
    //     //         .clone()],
    //     // );
    //     {
    //         let ext = self.externs.get_mut();
    //         // ext.deferred.depth_constants = Vec4::new(0.0, 1. / 0.01, 0.0, 0.0);
    //         ext.deferred.deferred_depth = self.gbuffers.uber_depth_eighth.into();
    //         let volumetrics = self.surfaces.get(self.lighting.volumetrics_rt0);
    //         ext.view.derive_matrices(volumetrics.resolution());
    //     }
    //     self.globals.scopes.view.bind(cmd).unwrap();
    //     self.globals.scopes.transparent.bind(cmd).unwrap();

    //     self.lighting.bind_volumetrics(self, cmd);
    //     cmd.state = PipelineState::new(Some(8), None, Some(2), Some(2));
    //     {
    //         cmd_event_span!(cmd, "volumetrics");
    //         self.submit_stage(
    //             cmd,
    //             RenderStage::Volumetrics,
    //             FeatureRendererSubscription::all(),
    //         );
    //     }

    //     {
    //         let volumetrics_upres = self.surfaces.get(self.lighting.volumetrics_upres);
    //         let ext = self.externs.get_mut();
    //         ext.view.derive_matrices(volumetrics_upres.resolution());

    //         ext.deferred.deferred_depth = self.gbuffers.uber_depth_half.into();
    //         ext.postprocess = externs::Postprocess {
    //             unk08: self.lighting.volumetrics_rt1.into(),
    //             unk00: self.lighting.volumetrics_rt0.into(),
    //             unk18: self.gbuffers.uber_depth_eighth.into(),
    //             res_for_unk00: self
    //                 .surfaces
    //                 .get(self.lighting.volumetrics_rt0)
    //                 .resolution_with_recip(),
    //             output_res: volumetrics_upres.resolution_with_recip(),
    //             ..Default::default()
    //         }
    //     }
    //     self.globals.scopes.view.bind(cmd).unwrap();
    //     self.bind_surfaces(cmd, &[self.lighting.volumetrics_upres], None);
    //     cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
    //     self.execute_global_pipeline(
    //         cmd,
    //         &self.globals.pipelines.volumetrics_upres_1,
    //         "volumetrics_upres_1",
    //     );

    //     // Rebind full resolution depth buffer
    //     {
    //         let shading_result = self.surfaces.get(self.shading_result);
    //         let ext = self.externs.get_mut();
    //         ext.deferred.deferred_depth = self.gbuffers.depth_proxy.lock().srv.clone().into();
    //         ext.view.derive_matrices(shading_result.resolution());
    //     }
    //     self.globals.scopes.view.bind(cmd).unwrap();
    // }

    // pub(super) fn apply_volume_fog(&self, cmd: &mut CommandList) {
    //     profiling::scope!("apply_volume_fog");

    //     {
    //         let shading_result = self.surfaces.get(self.shading_result);
    //         let ext = self.externs.get_mut();
    //         ext.deferred.deferred_depth = self.gbuffers.depth_proxy.lock().srv.clone().into();
    //         ext.view.derive_matrices(shading_result.resolution());

    //         ext.postprocess = externs::Postprocess {
    //             unk00: self.lighting.volumetrics_upres.into(),
    //             res_for_unk00: self
    //                 .surfaces
    //                 .get(self.lighting.volumetrics_upres)
    //                 .resolution_with_recip(),
    //             output_res: shading_result.resolution_with_recip(),
    //             ..Default::default()
    //         }
    //     }

    //     self.globals.scopes.view.bind(cmd).unwrap();
    //     self.bind_surfaces(cmd, &[self.shading_result], None);
    //     cmd.state = PipelineState::new(Some(5), Some(0), Some(0), Some(0));
    //     self.execute_global_pipeline(
    //         cmd,
    //         &self.globals.pipelines.copy_texture_bilinear,
    //         "copy_texture_bilinear (apply volumetrics)",
    //     );
    // }

    // fn compute_ssao(&self, cmd: &mut CommandList) {
    //     cmd_event_span!(cmd, "compute_ssao");
    //     {
    //         let ext = self.externs.get_mut();
    //         let main_res = self
    //             .surfaces
    //             .get(self.shading_result)
    //             .resolution_with_recip();
    //         ext.ssao = externs::Ssao {
    //             unk08: self.gbuffers.depth_proxy.lock().srv.clone().into(),
    //             unk20: main_res,
    //             unk30: main_res,
    //             unk40: main_res,
    //             unk80: ext.deferred.depth_constants,
    //             ..Default::default()
    //         };
    //     }

    //     self.bind_surfaces(cmd, &[self.lighting.ssao], None);
    //     cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
    //     self.execute_global_pipeline(
    //         cmd,
    //         &self.globals.pipelines.ssao_compute_ao_3D_ps,
    //         "ssao_compute_ao_3D_ps",
    //     );

    //     {
    //         let ext = self.externs.get_mut();
    //         ext.ssao.unk00 = self.lighting.ssao.into();
    //         ext.ssao.unka0_bilateral_blur = Vec4::new(2.40, 0.0, 0.0, 0.0);
    //     }
    //     self.bind_surfaces(cmd, &[self.lighting.ssao_pong], None);
    //     self.execute_global_pipeline(
    //         cmd,
    //         &self.globals.pipelines.ssao_bilateral_filter,
    //         "ssao_bilateral_filter (horizontal)",
    //     );

    //     {
    //         let ext = self.externs.get_mut();
    //         ext.ssao.unk00 = self.lighting.ssao_pong.into();
    //         ext.ssao.unka0_bilateral_blur = Vec4::new(0.0, 2.40, 0.0, 0.0);
    //     }
    //     self.bind_surfaces(cmd, &[self.lighting.ssao], None);
    //     self.execute_global_pipeline(
    //         cmd,
    //         &self.globals.pipelines.ssao_bilateral_filter,
    //         "ssao_bilateral_filter (vertical)",
    //     );
    // }
}
