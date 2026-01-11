use std::sync::Arc;

use alkahest_data::tfx::PipelineState;
use glam::{Vec4, vec4};

use crate::{
    Renderer,
    asset::{Handle, texture::Texture},
    gpu::command_list::CommandList,
    tfx::{externs, view::View},
};

#[derive(Default, Clone)]
pub struct AtmosphereData {
    pub atmosphere_lookup_near_0: Handle<Texture>,
    pub atmosphere_lookup_far_0: Handle<Texture>,
    pub atmosphere_lookup_near_1: Handle<Texture>,
    pub atmosphere_lookup_far_1: Handle<Texture>,

    pub atmosphere_lookup_vertical: Handle<Texture>,
}

impl Renderer {
    pub(crate) fn submit_atmosphere(self: &Arc<Self>, cmd: &mut CommandList, view: &View) {
        self.generate_sky_mask(cmd, view);

        self.generate_sky_lookup(cmd, view);
    }

    fn generate_sky_mask(self: &Arc<Self>, cmd: &mut CommandList, view: &View) {
        // Generate initial sky mask from uber depth
        {
            let sky_mask_surf = view.surfaces().get(view.atmosphere.sky_mask_initial);
            let uber_depth_surf = view.surfaces().get(view.gbuffers.uber_depth_half);
            sky_mask_surf.bind_single(cmd);
            let ext = self.externs.get_mut();
            ext.postprocess.input = view.gbuffers.uber_depth_half.into();
            ext.postprocess.res_for_input = uber_depth_surf.resolution_with_recip();
            ext.postprocess.output_res = sky_mask_surf.resolution_with_recip();
        }

        {
            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.sky_generate_sky_mask,
                "sky_generate_sky_mask",
            );
        }

        // Downsample 1/4th -> 1/8th
        {
            let sky_mask_initial_surf = view.surfaces().get(view.atmosphere.sky_mask_initial);
            let sky_mask_surf = view.surfaces().get(view.atmosphere.sky_mask);
            sky_mask_surf.bind_single(cmd);
            let ext = self.externs.get_mut();
            *ext.postprocess = externs::Postprocess {
                input: view.atmosphere.sky_mask_initial.into(),
                res_for_input: sky_mask_initial_surf.resolution_with_recip(),
                output_res: sky_mask_surf.resolution_with_recip(),
                unkc0: Vec4::ZERO,
                ..Default::default()
            };
        }

        {
            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.downsample_block_2x2,
                "downsample_block_2x2",
            );
        }

        // // Blur pass 1
        // {
        //     let sky_mask_surf = view.surfaces().get(view.atmosphere.sky_mask);
        //     let sky_mask_surf_temp = view.surfaces().get(view.atmosphere.sky_mask_temp);
        //     sky_mask_surf_temp.bind_single(cmd);
        //     let ext = self.externs.get_mut();
        //     *ext.postprocess = externs::Postprocess {
        //         input: view.atmosphere.sky_mask.into(),
        //         res_for_input: sky_mask_surf.resolution_with_recip(),
        //         output_res: sky_mask_surf_temp.resolution_with_recip(),
        //         unkc0: vec4(0.59197, 0.32437, 0.03, 0.022),
        //         unkd0: Vec4::ONE,
        //         ..Default::default()
        //     };
        // }

        // {
        //     cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
        //     self.execute_global_pipeline(
        //         cmd,
        //         &self.globals.pipelines.radial_blur_8,
        //         "radial_blur_8",
        //     );
        // }

        // // Blur pass 2
        // {
        //     let sky_mask_surf = view.surfaces().get(view.atmosphere.sky_mask);
        //     let sky_mask_surf_temp = view.surfaces().get(view.atmosphere.sky_mask_temp);
        //     sky_mask_surf.bind_single(cmd);
        //     let ext = self.externs.get_mut();
        //     *ext.postprocess = externs::Postprocess {
        //         input: view.atmosphere.sky_mask_temp.into(),
        //         res_for_input: sky_mask_surf_temp.resolution_with_recip(),
        //         output_res: sky_mask_surf.resolution_with_recip(),
        //         unkc0: vec4(0.59197, 0.32437, 0.08, 0.05867),
        //         unkd0: Vec4::NEG_ONE,
        //         ..Default::default()
        //     };
        // }

        // {
        //     cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
        //     self.execute_global_pipeline(
        //         cmd,
        //         &self.globals.pipelines.radial_blur_8,
        //         "radial_blur_8",
        //     );
        // }
    }

    fn generate_sky_lookup(self: &Arc<Self>, cmd: &mut CommandList, view: &View) {
        {
            let atm = &self.frame_packet.read().misc.atmosphere;
            let ext = self.externs.get_mut();
            ext.atmosphere.unka0 = view.atmosphere.sky_mask.into();

            ext.atmosphere.unk40 = atm.atmosphere_lookup_near_0.clone().into();
            ext.atmosphere.unk58 = atm.atmosphere_lookup_near_1.clone().into();

            ext.postprocess.unkc0 = vec4(-0.08323, 0.00, -0.99653, 1.0);
            ext.postprocess.unkd0 = vec4(-0.03088, 0.99952, 0.00258, 1.0);
            ext.postprocess.unke0 = vec4(0.99605, 0.03098, -0.08319, 1.0);
        }

        {
            self.bind_surfaces(cmd, &[view.atmosphere.sky_lookup_near], None);
            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.sky_lookup_generate_near,
                "sky_lookup_generate_near",
            );
        }

        {
            let atm = &self.frame_packet.read().misc.atmosphere;
            let ext = self.externs.get_mut();
            ext.atmosphere.unk40 = atm.atmosphere_lookup_far_0.clone().into();
            ext.atmosphere.unk58 = atm.atmosphere_lookup_far_1.clone().into();
        }

        {
            self.bind_surfaces(cmd, &[view.atmosphere.sky_lookup_far], None);
            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            self.execute_global_pipeline(
                cmd,
                &self.globals.pipelines.sky_lookup_generate_far,
                "sky_lookup_generate_far",
            );
        }
    }
}
