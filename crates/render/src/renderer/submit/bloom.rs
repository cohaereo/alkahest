use alkahest_data::tfx::PipelineState;
use glam::Vec4;

use crate::{
    cmd_event_span,
    gpu::command_list::CommandList,
    tfx::externs::{Postprocess, PostprocessInitialDownsample},
    Renderer,
};

impl Renderer {
    pub(super) fn submit_bloom(&self, cmd: &mut CommandList) {
        cmd_event_span!(cmd, "bloom");

        // TODO(cohae): Abstract postprocess extern initialization
        {
            let ext = &mut self.externs.get_mut();
            ext.postprocess = Postprocess {
                unk00: self.shading_result.into(),
                res_for_unk00: self
                    .surfaces
                    .get(self.shading_result)
                    .resolution_with_recip(),
                unkb0: Vec4::new(0.00, 0.0005, 0.016, 0.016),
                ..Default::default()
            };

            ext.postprocess_initial_downsample = PostprocessInitialDownsample {
                distortion: self.lighting.distortion.into(),
                ..Default::default()
            };
        }

        {
            self.bind_surfaces(cmd, &[self.bloom.initial_downsample], None);
            cmd.output_merger_set_depth_stencil_state(None, 0);

            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.bloom_initial_downsample_block_2x2,
                "bloom_initial_downsample_block_2x2",
            );
        }

        // Initial -> 6th
        {
            let ext = &mut self.externs.get_mut();
            ext.postprocess = Postprocess {
                unk00: self.bloom.initial_downsample.into(),
                res_for_unk00: self
                    .surfaces
                    .get(self.bloom.initial_downsample)
                    .resolution_with_recip(),
                unkb0: Vec4::W,
                output_res: self
                    .surfaces
                    .get(self.bloom.downsample_6th)
                    .resolution_with_recip(),
                ..Default::default()
            };
        }

        {
            self.bind_surfaces(cmd, &[self.bloom.downsample_6th], None);
            cmd.output_merger_set_depth_stencil_state(None, 0);

            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.downsample_block_2x2_with_nan_kill,
                "downsample_block_2x2_with_nan_kill",
            );
        }

        // 6th -> 12th
        {
            let ext = &mut self.externs.get_mut();
            ext.postprocess = Postprocess {
                unk00: self.bloom.downsample_6th.into(),
                res_for_unk00: self
                    .surfaces
                    .get(self.bloom.downsample_6th)
                    .resolution_with_recip(),
                unkb0: Vec4::W,
                output_res: self
                    .surfaces
                    .get(self.bloom.downsample_12th)
                    .resolution_with_recip(),
                ..Default::default()
            };
        }

        {
            self.bind_surfaces(cmd, &[self.bloom.downsample_12th], None);
            cmd.output_merger_set_depth_stencil_state(None, 0);

            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.downsample_block_2x2_with_nan_kill,
                "downsample_block_2x2_with_nan_kill",
            );
        }

        // 12th -> 24th
        {
            let ext = &mut self.externs.get_mut();
            ext.postprocess = Postprocess {
                unk00: self.bloom.downsample_12th.into(),
                res_for_unk00: self
                    .surfaces
                    .get(self.bloom.downsample_12th)
                    .resolution_with_recip(),
                unkb0: Vec4::W,
                output_res: self
                    .surfaces
                    .get(self.bloom.downsample_24th)
                    .resolution_with_recip(),
                ..Default::default()
            };
        }

        {
            self.bind_surfaces(cmd, &[self.bloom.downsample_24th], None);
            cmd.output_merger_set_depth_stencil_state(None, 0);

            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.downsample_block_2x2_with_nan_kill,
                "downsample_block_2x2_with_nan_kill",
            );
        }

        // Sample columns for autoexposure
        {
            let ext = &mut self.externs.get_mut();
            ext.postprocess = Postprocess {
                unk00: self.bloom.downsample_24th.into(),
                output_res: self
                    .surfaces
                    .get(self.bloom.autoexposure_sample_columns)
                    .resolution_with_recip(),
                unkb0: Vec4::new(0.01, 0.90, 1.00, 1.00),
                ..Default::default()
            };
        }

        {
            self.bind_surfaces(cmd, &[self.bloom.autoexposure_sample_columns], None);
            cmd.output_merger_set_depth_stencil_state(None, 0);

            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.autoexposure_sample_columns,
                "autoexposure_sample_columns",
            );

            self.bloom.autoexposure_sample_columns_cpu.lock().update(
                &cmd,
                self.surfaces.get(self.bloom.autoexposure_sample_columns),
            );
        }
    }
}
