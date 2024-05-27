use alkahest_data::{
    technique::StateSelection,
    tfx::{TfxRenderStage, TfxShaderStage},
};

use crate::{
    ecs::Scene,
    gpu_event,
    renderer::Renderer,
    tfx::{externs, externs::ExternDefault, scope::ScopeTransparentAdvanced},
};

impl Renderer {
    pub fn draw_transparents_pass(&self, scene: &Scene) {
        gpu_event!(self.gpu, "transparents_pass");

        {
            let mut data = self.data.lock();
            let existing_transparent = data
                .externs
                .transparent
                .as_ref()
                .cloned()
                .unwrap_or(ExternDefault::extern_default());

            data.externs.transparent = Some(externs::Transparent {
                unk00: data.gbuffers.atmos_ss_far_lookup.view.clone().into(),
                // TODO(cohae): unk08 and unk18 are actually the downsampling of their respective lookup
                unk08: data.gbuffers.atmos_ss_far_lookup.view.clone().into(),
                unk10: data.gbuffers.atmos_ss_near_lookup.view.clone().into(),
                unk18: data.gbuffers.atmos_ss_near_lookup.view.clone().into(),
                unk20: self.gpu.light_grey_texture.view.clone().into(),
                // unk20: data.gbuffers.staging_clone.view.clone().into(),
                unk28: self.gpu.light_grey_texture.view.clone().into(),
                unk30: self.gpu.light_grey_texture.view.clone().into(),
                unk38: self.gpu.light_grey_texture.view.clone().into(),
                unk40: self.gpu.light_grey_texture.view.clone().into(),
                // unk48: gctx.black_texture.view.clone().into(),
                unk48: data.gbuffers.shading_result_read.view.clone().into(),
                unk50: self.gpu.black_texture.view.clone().into(),
                unk58: self.gpu.light_grey_texture.view.clone().into(),
                unk60: data.gbuffers.shading_result_read.view.clone().into(),
                ..existing_transparent
            });

            // TODO(cohae): Write an abstraction for native-initialized scopes
            if let Some(ta_cb) = self
                .render_globals
                .scopes
                .transparent_advanced
                .stage_pixel
                .as_ref()
                .unwrap()
                .cbuffer
                .as_ref()
            {
                assert!(
                    std::mem::size_of_val(ta_cb.data_array())
                        >= std::mem::size_of::<ScopeTransparentAdvanced>()
                );

                unsafe {
                    (ta_cb.data_array().as_ptr() as *mut ScopeTransparentAdvanced)
                        .write(ScopeTransparentAdvanced::default());
                    let slot = self
                        .render_globals
                        .scopes
                        .transparent_advanced
                        .stage_pixel
                        .as_ref()
                        .unwrap()
                        .stage
                        .constants
                        .constant_buffer_slot as u32;

                    ta_cb.bind(slot, TfxShaderStage::Pixel);
                    ta_cb.bind(slot, TfxShaderStage::Vertex);
                    ta_cb.bind(slot, TfxShaderStage::Compute);
                }
            }

            unsafe {
                self.gpu.context().OMSetRenderTargets(
                    Some(&[Some(data.gbuffers.shading_result.render_target.clone())]),
                    &data.gbuffers.depth.view,
                );

                self.gpu
                    .context()
                    .OMSetDepthStencilState(&data.gbuffers.depth.state_readonly, 0);
            }
        }

        self.gpu
            .current_states
            .store(StateSelection::new(Some(8), Some(15), Some(2), Some(1)));

        self.run_renderstage_systems(scene, TfxRenderStage::DecalsAdditive);

        {
            let gbuffers = &self.data.lock().gbuffers;
            gbuffers
                .shading_result
                .copy_to(&gbuffers.shading_result_read);
        }

        self.gpu
            .current_states
            .store(StateSelection::new(Some(8), Some(15), Some(2), Some(1)));
        self.render_globals.scopes.transparent.bind(self).unwrap();

        self.run_renderstage_systems(scene, TfxRenderStage::Transparents);

        {
            let gbuffers = &self.data.lock().gbuffers;
            gbuffers
                .shading_result
                .copy_to(&gbuffers.shading_result_read);
        }

        // draw_utilities(self, scene);
    }
}
