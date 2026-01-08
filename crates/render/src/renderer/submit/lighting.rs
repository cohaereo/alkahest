use std::sync::Arc;

use alkahest_data::tfx::{FeatureRendererSubscription, PipelineState, RenderStage};

use super::Renderer;
use crate::{
    cmd_event_span,
    gpu::command_list::CommandList,
    renderer::submit::geometry::GeometryCommandLists,
    tfx::{externs, view::View},
};

impl Renderer {
    pub(super) fn submit_lighting(
        self: &Arc<Self>,
        cmd: &mut CommandList,
        view: &View,
        geo: Option<&GeometryCommandLists>,
    ) {
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
            self.submit_stage(
                cmd,
                RenderStage::Cubemaps,
                FeatureRendererSubscription::all(),
            );
        }
        {
            cmd_event_span!(cmd, "local_lights");
            let _gpuspan = self.profiler.scope(cmd, "local_lights");

            view.lighting.bind_diffuse_specular(cmd, &view.surfaces);
            cmd.state = PipelineState::new(Some(8), None, Some(2), Some(2));
            if let Some(geo) = geo {
                let (sync_job, set) = &geo.lighting;
                sync_job.wait();
                self.cmd_pool.finish(cmd, *set);
            } else {
                view.lighting.bind_diffuse_specular(cmd, &view.surfaces);
                cmd.state = PipelineState::new(Some(8), None, Some(2), Some(2));
                cmd.flush_states();
                self.submit_stage(
                    cmd,
                    RenderStage::LightingApply,
                    FeatureRendererSubscription::all(),
                );
            }
        }

        if self.settings().volumetrics {
            self.submit_volumetrics(cmd, view, geo);
        }
    }

    pub(super) fn submit_volumetrics(
        self: &Arc<Self>,
        cmd: &mut CommandList,
        view: &View,
        geo: Option<&GeometryCommandLists>,
    ) {
        profiling::scope!("submit_volumetrics");
        let _gpuspan = self.profiler.scope(cmd, "submit_volumetrics");

        // cmd.pixel_set_shader_resources(
        //     10,
        //     &[self
        //         .surfaces
        //         .get(view.gbuffers.uber_depth_eigth)
        //         .srv
        //         .clone()],
        // );
        {
            let ext = self.externs.get_mut();
            // ext.deferred.depth_constants = Vec4::new(0.0, 1. / 0.01, 0.0, 0.0);
            ext.deferred.deferred_depth = view.gbuffers.uber_depth_eighth.into();
            let volumetrics = view.surfaces.get(view.lighting.volumetrics_rt0);
            ext.view.derive_matrices(volumetrics.resolution());
        }
        self.globals.scopes.view.bind(cmd).unwrap();
        self.globals.scopes.transparent.bind(cmd).unwrap();

        view.lighting.bind_volumetrics(self, cmd);
        cmd.state = PipelineState::new(Some(8), None, Some(2), Some(2));
        {
            cmd_event_span!(cmd, "volumetrics");

            if self.settings().multithreading {
                self.submit_stage_parallel_linear(
                    cmd,
                    RenderStage::Volumetrics,
                    FeatureRendererSubscription::all(),
                );
            } else {
                self.submit_stage(
                    cmd,
                    RenderStage::Volumetrics,
                    FeatureRendererSubscription::all(),
                );
            }
        }

        {
            let volumetrics_upres = view.surfaces.get(view.lighting.volumetrics_upres);
            let ext = self.externs.get_mut();
            ext.view.derive_matrices(volumetrics_upres.resolution());

            ext.deferred.deferred_depth = view.gbuffers.uber_depth_half.into();
            ext.postprocess = externs::Postprocess {
                unk08: view.lighting.volumetrics_rt1.into(),
                unk00: view.lighting.volumetrics_rt0.into(),
                unk18: view.gbuffers.uber_depth_eighth.into(),
                res_for_unk00: view
                    .surfaces
                    .get(view.lighting.volumetrics_rt0)
                    .resolution_with_recip(),
                output_res: volumetrics_upres.resolution_with_recip(),
                ..Default::default()
            }
            .into();
        }
        self.globals.scopes.view.bind(cmd).unwrap();
        self.bind_surfaces(cmd, &[view.lighting.volumetrics_upres], None);
        cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
        self.execute_global_pipeline(
            cmd,
            &self.globals.pipelines.volumetrics_upres_1,
            "volumetrics_upres_1",
        );

        // Rebind full resolution depth buffer
        {
            let shading_result = view.surfaces.get(view.shading_result);
            let ext = self.externs.get_mut();
            ext.deferred.deferred_depth = view.gbuffers.depth_proxy.lock().srv.clone().into();
            ext.view.derive_matrices(shading_result.resolution());
        }
        self.globals.scopes.view.bind(cmd).unwrap();
    }

    pub(super) fn apply_volume_fog(&self, cmd: &mut CommandList, view: &View) {
        profiling::scope!("apply_volume_fog");

        {
            let shading_result = view.surfaces.get(view.shading_result);
            let ext = self.externs.get_mut();
            ext.deferred.deferred_depth = view.gbuffers.depth_proxy.lock().srv.clone().into();
            ext.view.derive_matrices(shading_result.resolution());

            ext.postprocess = externs::Postprocess {
                unk00: view.lighting.volumetrics_upres.into(),
                res_for_unk00: view
                    .surfaces
                    .get(view.lighting.volumetrics_upres)
                    .resolution_with_recip(),
                output_res: shading_result.resolution_with_recip(),
                ..Default::default()
            }
            .into();
        }

        self.globals.scopes.view.bind(cmd).unwrap();
        self.bind_surfaces(cmd, &[view.shading_result], None);
        cmd.state = PipelineState::new(Some(5), Some(0), Some(0), Some(0));
        self.execute_global_pipeline(
            cmd,
            &self.globals.pipelines.copy_texture_bilinear,
            "copy_texture_bilinear (apply volumetrics)",
        );
    }

    // fn compute_ssao(&self, cmd: &mut CommandList) {
    //     cmd_event_span!(cmd, "compute_ssao");
    //     {
    //         let ext = self.externs.get_mut();
    //         let main_res = self
    //             .surfaces
    //             .get(view.shading_result)
    //             .resolution_with_recip();
    //         ext.ssao = externs::Ssao {
    //             unk08: view.gbuffers.depth_proxy.lock().srv.clone().into(),
    //             unk20: main_res,
    //             unk30: main_res,
    //             unk40: main_res,
    //             unk80: ext.deferred.depth_constants,
    //             ..Default::default()
    //         };
    //     }

    //     self.bind_surfaces(cmd, &[view.lighting.ssao], None);
    //     cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
    //     self.execute_global_pipeline(
    //         cmd,
    //         &self.globals.pipelines.ssao_compute_ao_3D_ps,
    //         "ssao_compute_ao_3D_ps",
    //     );

    //     {
    //         let ext = self.externs.get_mut();
    //         ext.ssao.unk00 = view.lighting.ssao.into();
    //         ext.ssao.unka0_bilateral_blur = Vec4::new(2.40, 0.0, 0.0, 0.0);
    //     }
    //     self.bind_surfaces(cmd, &[view.lighting.ssao_pong], None);
    //     self.execute_global_pipeline(
    //         cmd,
    //         &self.globals.pipelines.ssao_bilateral_filter,
    //         "ssao_bilateral_filter (horizontal)",
    //     );

    //     {
    //         let ext = self.externs.get_mut();
    //         ext.ssao.unk00 = view.lighting.ssao_pong.into();
    //         ext.ssao.unka0_bilateral_blur = Vec4::new(0.0, 2.40, 0.0, 0.0);
    //     }
    //     self.bind_surfaces(cmd, &[view.lighting.ssao], None);
    //     self.execute_global_pipeline(
    //         cmd,
    //         &self.globals.pipelines.ssao_bilateral_filter,
    //         "ssao_bilateral_filter (vertical)",
    //     );
    // }
}
