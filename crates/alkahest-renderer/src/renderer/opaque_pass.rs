use alkahest_data::{technique::StateSelection, tfx::TfxRenderStage};
use glam::Vec4;

use crate::{
    ecs::Scene,
    gpu_event,
    renderer::Renderer,
    tfx::{externs, externs::ExternDefault},
};

impl Renderer {
    pub fn draw_opaque_pass(&self, scene: &Scene) {
        gpu_event!(self.gpu, "generate_gbuffer");

        self.gpu
            .current_states
            .store(StateSelection::new(Some(0), Some(2), Some(2), Some(0)));

        unsafe {
            let gbuffers = &self.data.lock().gbuffers;
            self.gpu.context().OMSetRenderTargets(
                Some(&[
                    Some(gbuffers.rt0.render_target.clone()),
                    Some(gbuffers.rt1.render_target.clone()),
                    Some(gbuffers.rt2.render_target.clone()),
                ]),
                &gbuffers.depth.view,
            );
            // self.gpu
            //     .context()
            //     .OMSetDepthStencilState(&gbuffers.depth.state, 0);

            gbuffers.rt0.clear(&[0.0, 0.0, 0.0, 0.0]);
            gbuffers.rt1.clear(&[0.0, 0.0, 0.0, 0.0]);
            gbuffers.rt2.clear(&[1.0, 0.5, 1.0, 0.0]);
            gbuffers.depth.clear(0.0, 0);
        }

        // Draw opaque pass
        self.run_renderstage_systems(scene, TfxRenderStage::GenerateGbuffer);
        {
            let mut data = self.data.lock();

            data.externs.deferred = Some(externs::Deferred {
                depth_constants: Vec4::new(0.0, 1. / 0.0001, 0.0, 0.0),
                deferred_depth: data.gbuffers.depth.texture_copy_view.clone().into(),
                deferred_rt0: data.gbuffers.rt0.view.clone().into(),
                deferred_rt1: data.gbuffers.rt1_read.view.clone().into(),
                deferred_rt2: data.gbuffers.rt2.view.clone().into(),
                light_diffuse: data.gbuffers.light_diffuse.view.clone().into(),
                light_specular: data.gbuffers.light_specular.view.clone().into(),
                light_ibl_specular: data.gbuffers.light_ibl_specular.view.clone().into(),
                // unk98: gctx.light_grey_texture.view.clone().into(),
                // unk98: data.gbuffers.staging_clone.view.clone().into(),
                sky_hemisphere_mips: self.gpu.sky_hemisphere_placeholder.view.clone().into(),
                ..ExternDefault::extern_default()
            });
            data.gbuffers.rt1.copy_to(&data.gbuffers.rt1_read);
            data.gbuffers.depth.copy_depth();

            data.externs.decal = Some(externs::Decal {
                unk08: data.gbuffers.rt1_read.view.clone().into(),
                ..Default::default()
            });
        }

        self.gpu
            .current_states
            .store(StateSelection::new(Some(8), Some(15), Some(2), Some(1)));
        self.run_renderstage_systems(scene, TfxRenderStage::Decals);
    }
}
