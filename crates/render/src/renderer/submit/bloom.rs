use alkahest_data::tfx::PipelineState;
use glam::{Vec4, vec4};

use crate::{
    Renderer, cmd_event_span,
    gpu::command_list::CommandList,
    renderer::surface::SurfaceHandle,
    tfx::{externs::PostprocessInitialDownsample, view::View},
};

impl Renderer {
    pub(super) fn submit_bloom(&self, cmd: &mut CommandList, view: &View) {
        cmd_event_span!(cmd, "bloom");
        let _gpu_span = self.profiler.scope(cmd, "bloom");

        if !view.settings.bloom {
            return;
        }

        cmd.output_merger_set_depth_stencil_state(None, 0);
        cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));

        let bind_postprocessing = |cmd: &mut CommandList,
                                   surface: SurfaceHandle,
                                   out_surface: SurfaceHandle,
                                   unkc0: Vec4| {
            let surf = view.surfaces.get(surface);
            cmd.pixel_set_shader_resources(0, &[surf.srv.as_ref()]);

            let ext = &mut self.externs.get_mut();
            ext.postprocess.input = surface.into();
            ext.postprocess.res_for_input = surf.resolution_with_recip();
            ext.postprocess.output_res = view.surfaces.get(out_surface).resolution_with_recip();
            ext.postprocess.unkc0 = unkc0;
            self.bind_surfaces(cmd, &[out_surface], None);
        };

        let bind_scope = |cmd: &mut CommandList,
                          in_surface: SurfaceHandle,
                          out_surface: SurfaceHandle,
                          unk3: Vec4,
                          unk4: Vec4,
                          unk5: Vec4,
                          unk6: Vec4,
                          unk7: Vec4| {
            let in_surf = view.surfaces.get(in_surface);
            let out_surf = view.surfaces.get(out_surface);
            cmd.pixel_set_shader_resources(0, &[in_surf.srv.as_ref()]);
            self.postprocess_cbuffer
                .write(
                    cmd,
                    &PostProcessScope {
                        in_res: in_surf.resolution_with_recip(),
                        out_res: out_surf.resolution_with_recip(),
                        unk2: Vec4::ZERO,
                        unk3,
                        unk4,
                        unk5,
                        unk6,
                        unk7,
                    },
                )
                .ok();
            self.postprocess_cbuffer
                .bind(cmd, alkahest_data::tfx::ShaderStage::Pixel, 11);
        };

        let blur = |cmd: &mut CommandList,
                    in_surface: SurfaceHandle,
                    temp_surface: SurfaceHandle,
                    variant: BlurVariant,
                    strip_alpha: bool,
                    horz_unk3: Vec4,
                    horz_unk4: Vec4,
                    horz_unk5: Vec4,
                    vert_unk3: Vec4,
                    vert_unk4: Vec4,
                    vert_unk5: Vec4| {
            bind_postprocessing(cmd, in_surface, temp_surface, Vec4::ZERO);
            bind_scope(
                cmd,
                in_surface,
                temp_surface,
                horz_unk3,
                horz_unk4,
                horz_unk5,
                Vec4::ONE,
                Vec4::ZERO,
            );

            match variant {
                BlurVariant::Gaussian10 => {
                    self.execute_global_pipeline(
                        cmd,
                        &self.globals.pipelines.gaussian_10_horz,
                        "gaussian_10_horz",
                    );
                }
                BlurVariant::Weighted6 => {
                    self.execute_global_pipeline(
                        cmd,
                        &self.globals.pipelines.weighted_6_horz,
                        "weighted_6_horz",
                    );
                }
            }

            bind_postprocessing(cmd, temp_surface, in_surface, Vec4::ZERO);
            bind_scope(
                cmd,
                temp_surface,
                in_surface,
                vert_unk3,
                vert_unk4,
                vert_unk5,
                if strip_alpha {
                    Vec4::new(1.0, 1.0, 1.0, 0.0)
                } else {
                    Vec4::ONE
                },
                if strip_alpha { Vec4::W } else { Vec4::ZERO },
            );

            match variant {
                BlurVariant::Gaussian10 => {
                    self.execute_global_pipeline(
                        cmd,
                        &self.globals.pipelines.gaussian_10_vert,
                        "gaussian_10_vert",
                    );
                }
                BlurVariant::Weighted6 => {
                    self.execute_global_pipeline(
                        cmd,
                        &self.globals.pipelines.weighted_6_vert,
                        "weighted_6_vert",
                    );
                }
            }
        };

        {
            bind_postprocessing(
                cmd,
                view.shading_result,
                view.bloom.bloom_3rd,
                vec4(0.00, 0.0005, 0.016, 0.016),
            );

            self.externs.get_mut().postprocess_initial_downsample = PostprocessInitialDownsample {
                distortion: view.lighting.distortion.into(),
                ..Default::default()
            }
            .into();

            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.bloom_initial_downsample_block_2x2,
                "bloom_initial_downsample_block_2x2",
            );
        }

        {
            bind_postprocessing(cmd, view.bloom.bloom_3rd, view.bloom.bloom_6th, Vec4::W);

            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.downsample_block_2x2_with_nan_kill,
                "downsample_block_2x2_with_nan_kill",
            );
        }

        {
            bind_postprocessing(cmd, view.bloom.bloom_6th, view.bloom.bloom_12th, Vec4::ZERO);

            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.downsample_block_2x2,
                "downsample_block_2x2",
            );
        }

        {
            bind_postprocessing(
                cmd,
                view.bloom.bloom_12th,
                view.bloom.bloom_24th,
                Vec4::ZERO,
            );

            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.downsample_block_2x2,
                "downsample_block_2x2",
            );
        }

        {
            bind_postprocessing(
                cmd,
                view.bloom.bloom_12th,
                view.bloom.bloom_12th_half_width,
                Vec4::ZERO,
            );

            bind_scope(
                cmd,
                view.bloom.bloom_12th,
                view.bloom.bloom_12th_half_width,
                vec4(0.12667, 0.37333, 0.00, 0.00),
                vec4(0.01793, 0.00547, 0.00, 0.00),
                vec4(0.00, 0.00, 0.00, 0.00),
                vec4(2.00, 2.00, 2.00, 1.00),
                Vec4::ZERO,
            );

            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.downsample_gaussian_8x1,
                "downsample_gaussian_8x1",
            );
        }

        {
            bind_postprocessing(
                cmd,
                view.bloom.bloom_12th_half_width,
                view.bloom.bloom_12th_quarter_width,
                Vec4::ZERO,
            );

            bind_scope(
                cmd,
                view.bloom.bloom_12th_half_width,
                view.bloom.bloom_12th_quarter_width,
                vec4(0.12667, 0.37333, 0.00, 0.00),
                vec4(0.03586, 0.01094, 0.00, 0.00),
                vec4(0.00, 0.00, 0.00, 0.00),
                vec4(2.00, 2.00, 2.00, 1.00),
                Vec4::ZERO,
            );

            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.downsample_gaussian_8x1,
                "downsample_gaussian_8x1",
            );
        }

        {
            bind_postprocessing(
                cmd,
                view.bloom.bloom_12th_quarter_width,
                view.bloom.bloom_12th_quarter_width_temp,
                Vec4::ZERO,
            );

            bind_scope(
                cmd,
                view.bloom.bloom_12th_quarter_width,
                view.bloom.bloom_12th_quarter_width_temp,
                vec4(0.04734, 0.0858, 0.14793, 0.21893),
                vec4(0.17344, 0.12284, 0.00, 0.00),
                vec4(0.07344, 0.02284, 0.00, 0.00),
                vec4(2.00, 2.00, 2.00, 1.00),
                Vec4::ZERO,
            );

            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.downsample_gaussian_16x1,
                "downsample_gaussian_16x1",
            );
        }

        {
            bind_postprocessing(
                cmd,
                view.bloom.bloom_12th_quarter_width_temp,
                view.bloom.bloom_12th_quarter_width,
                Vec4::ZERO,
            );

            bind_scope(
                cmd,
                view.bloom.bloom_12th_quarter_width_temp,
                view.bloom.bloom_12th_quarter_width,
                vec4(0.04667, 0.08, 0.14, 0.23333),
                vec4(0.00, 0.00, 0.00, 0.00),
                vec4(0.00, 0.00, 0.00, 0.00),
                vec4(2.00, 2.00, 2.00, 1.00),
                Vec4::ZERO,
            );

            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.downsample_gaussian_16x1,
                "downsample_gaussian_16x1",
            );
        }

        blur(
            cmd,
            view.bloom.bloom_24th,
            view.bloom.bloom_24th_temp,
            BlurVariant::Gaussian10,
            true,
            vec4(0.05882, 0.17647, 0.52941, 0.00),
            vec4(-0.05625, -0.02917, -0.00625, 0.01111),
            vec4(0.01667, 0.04375, 0.00, 0.01111),
            //
            vec4(0.05882, 0.17647, 0.52941, 0.00),
            vec4(-0.10, -0.05185, -0.01111, 0.00625),
            vec4(0.02963, 0.07778, 0.00, 0.00625),
        );

        {
            {
                bind_postprocessing(
                    cmd,
                    view.bloom.bloom_24th,
                    view.bloom.bloom_12th_combined,
                    Vec4::ZERO,
                );
                let ext = &mut self.externs.get_mut();
                ext.postprocess.unk08 = view.bloom.bloom_12th.into();
                ext.postprocess.unkc0 = vec4(0.75, 1.30, 2.50, 1.00);
                ext.postprocess.unkd0 = vec4(0.64, 1.07, 2.14, 1.00);
                ext.postprocess.unke0 = vec4(1.00, 1.00, 0.00, 0.00);
                ext.postprocess.unkf0 = vec4(1.00, 1.00, 0.00, 0.00);
            }

            self.execute_global_pipeline(cmd, &self.globals.pipelines.weighted_add, "weighted_add");
        }

        blur(
            cmd,
            view.bloom.bloom_12th_combined,
            view.bloom.bloom_12th_temp,
            BlurVariant::Gaussian10,
            false,
            vec4(0.05882, 0.17647, 0.52941, 0.00),
            vec4(-0.02813, -0.01458, -0.00313, 0.00556),
            vec4(0.00833, 0.02187, 0.00, 0.00556),
            //
            vec4(0.05882, 0.17647, 0.52941, 0.00),
            vec4(-0.05, -0.02593, -0.00556, 0.00313),
            vec4(0.01481, 0.03889, 0.00, 0.00313),
        );

        {
            {
                bind_postprocessing(
                    cmd,
                    view.bloom.bloom_12th_combined,
                    view.bloom.bloom_6th_combined,
                    Vec4::ZERO,
                );
                let ext = &mut self.externs.get_mut();
                ext.postprocess.unk08 = view.bloom.bloom_6th.into();

                ext.postprocess.unkc0 = vec4(1.00, 1.00, 1.00, 1.00);
                ext.postprocess.unkd0 = vec4(1.80, 2.025, 2.40, 1.00);
                ext.postprocess.unke0 = vec4(1.00, 1.00, 0.00, 0.00);
                ext.postprocess.unkf0 = vec4(1.00, 1.00, 0.00, 0.00);
            }

            self.execute_global_pipeline(cmd, &self.globals.pipelines.weighted_add, "weighted_add");
        }

        blur(
            cmd,
            view.bloom.bloom_6th_combined,
            view.bloom.bloom_6th_temp,
            BlurVariant::Gaussian10,
            false,
            vec4(0.05882, 0.17647, 0.52941, 0.00),
            vec4(-0.01406, -0.00729, -0.00156, 0.00278),
            vec4(0.00417, 0.01094, 0.00, 0.00278),
            //
            vec4(0.05882, 0.17647, 0.52941, 0.00),
            vec4(-0.025, -0.01296, -0.00278, 0.00156),
            vec4(0.00741, 0.01944, 0.00, 0.00156),
        );

        {
            {
                bind_postprocessing(
                    cmd,
                    view.bloom.bloom_6th_combined,
                    view.bloom.bloom_3rd_combined,
                    Vec4::ZERO,
                );
                let ext = &mut self.externs.get_mut();
                ext.postprocess.unk08 = view.bloom.bloom_3rd.into();
                ext.postprocess.unk10 = view.bloom.bloom_12th_quarter_width.into();

                ext.postprocess.unkc0 = vec4(1.00, 1.00, 1.00, 1.00);
                ext.postprocess.unkd0 = vec4(2.75, 2.75, 2.75, 0.00);
                ext.postprocess.unke0 = vec4(0.01, 0.01, 0.02, 0.00);
            }

            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.combined_bloom_line_blur,
                "combined_bloom_line_blur",
            );
        }

        blur(
            cmd,
            view.bloom.bloom_3rd_combined,
            view.bloom.bloom_3rd_temp,
            BlurVariant::Weighted6,
            false,
            vec4(0.25, 0.50, 0.00, 0.00),
            vec4(0.00, -0.00359, -0.00078, 0.00139),
            vec4(0.00203, 0.00, 0.00, 0.00139),
            //
            vec4(0.25, 0.50, 0.00, 0.00),
            vec4(0.00, -0.00639, -0.00139, 0.00078),
            vec4(0.00361, 0.00, 0.00, 0.00078),
        );

        {
            bind_postprocessing(
                cmd,
                view.bloom.bloom_3rd_temp,
                view.bloom.bloom_final,
                Vec4::ONE,
            );

            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.copy_texture_bilinear,
                "copy_texture_bilinear",
            );
        }

        // // Sample columns for autoexposure
        // {
        //     let ext = &mut self.externs.get_mut();
        //     ext.postprocess = Postprocess {
        //         unk00: view.bloom.downsample_24th.into(),
        //         output_res: self
        //             .surfaces
        //             .get(view.bloom.autoexposure_sample_columns)
        //             .resolution_with_recip(),
        //         unkb0: Vec4::new(0.01, 0.90, 1.00, 1.00),
        //         ..Default::default()
        //     };
        // }

        // {
        //     self.bind_surfaces(cmd, &[view.bloom.autoexposure_sample_columns], None);
        //     cmd.output_merger_set_depth_stencil_state(None, 0);

        //     cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
        //     self.execute_global_pipeline(
        //         cmd,
        //         &self.globals.pipelines.autoexposure_sample_columns,
        //         "autoexposure_sample_columns",
        //     );

        //     view.bloom.autoexposure_sample_columns_cpu.lock().update(
        //         &cmd,
        //         view.surfaces.get(view.bloom.autoexposure_sample_columns),
        //     );
        // }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct PostProcessScope {
    out_res: Vec4,
    in_res: Vec4,
    unk2: Vec4,
    unk3: Vec4,
    unk4: Vec4,
    unk5: Vec4,
    unk6: Vec4,
    unk7: Vec4,
}

pub enum BlurVariant {
    Gaussian10,
    Weighted6,
}
