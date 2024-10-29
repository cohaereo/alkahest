use alkahest_data::technique::StateSelection;
use glam::Vec4;

use crate::{
    ecs::{map::MapAtmosphere, render::light::draw_light_system, Scene},
    gpu_event, gpu_profile_event,
    renderer::{cubemaps::draw_cubemap_system, Renderer},
    tfx::externs::{self, ExternDefault, ShadowMask},
};

impl Renderer {
    pub fn draw_lighting_pass(&self, scene: &mut Scene) {
        gpu_profile_event!(self.gpu, "lighting_pass");

        unsafe {
            let data = &mut self.data.lock();
            self.gpu.context().OMSetRenderTargets(
                Some(&[
                    Some(data.gbuffers.light_diffuse.render_target.clone()),
                    Some(data.gbuffers.light_specular.render_target.clone()),
                ]),
                None,
            );

            data.gbuffers
                .light_diffuse
                .clear(&[0.001, 0.001, 0.001, 0.0]);
            data.gbuffers.light_specular.clear(&[0.0, 0.0, 0.0, 0.0]);
            data.gbuffers
                .light_ibl_specular
                .clear(&[0.0, 0.0, 0.0, 0.0]);

            self.gpu
                .current_states
                .store(StateSelection::new(Some(8), Some(0), Some(2), Some(2)));

            data.externs.global_lighting =
                Some(data.externs.global_lighting.take().unwrap_or_default());

            data.externs.shadow_mask = Some(ShadowMask {
                unk00: self.gpu.white_texture.view.clone().into(),
                unk08: self.gpu.white_texture.view.clone().into(),
                unk10: self.gpu.white_texture.view.clone().into(),
                ..Default::default()
            })
        }

        {
            if self.settings.matcap {
                gpu_event!(self.gpu, "matcap");
                self.matcap.draw(self);
            } else {
                if self.settings.feature_global_lighting {
                    gpu_profile_event!(self.gpu, "global_lighting");

                    self.gpu.current_states.store(StateSelection::new(
                        Some(0),
                        Some(0),
                        Some(0),
                        Some(0),
                    ));

                    let pipeline = &self.render_globals.pipelines.global_lighting;
                    self.execute_global_pipeline(pipeline, "global_lighting");
                }

                self.gpu.current_states.store(StateSelection::new(
                    Some(8),
                    Some(0),
                    Some(2),
                    Some(2),
                ));

                {
                    gpu_profile_event!(self.gpu, "deferred_lights");
                    draw_light_system(self, scene)
                }

                if self.settings.feature_cubemaps {
                    unsafe {
                        let data = &mut self.data.lock();
                        self.gpu.context().OMSetRenderTargets(
                            Some(&[
                                Some(data.gbuffers.light_diffuse.render_target.clone()),
                                Some(data.gbuffers.light_ibl_specular.render_target.clone()),
                            ]),
                            None,
                        );
                    }

                    gpu_event!(self.gpu, "cubemaps");
                    draw_cubemap_system(self, scene);
                }
            }
        }

        {
            if self.settings.ssao {
                gpu_profile_event!(self.gpu, "ssao");
                self.ssao.draw(self);
            }
        }
    }

    // TODO(cohae): woe, naming conventions be upon ye
    pub fn draw_shading_pass(&self, scene: &Scene) {
        gpu_profile_event!(self.gpu, "shading_pass");

        unsafe {
            let gbuffers = &self.data.lock().gbuffers;
            self.gpu.context().OMSetRenderTargets(
                Some(&[Some(gbuffers.shading_result.render_target.clone()), None]),
                None,
            );
        }

        {
            gpu_event!(self.gpu, "deferred_shading");
            let pipeline = if scene.get_resource::<MapAtmosphere>().is_some()
                && self.settings.feature_atmosphere
            {
                &self.render_globals.pipelines.deferred_shading
            } else {
                &self.render_globals.pipelines.deferred_shading_no_atm
            };

            self.gpu
                .current_states
                .store(StateSelection::new(Some(0), Some(0), Some(0), Some(0)));
            self.execute_global_pipeline(pipeline, "deferred_shading");
        }
    }

    pub fn draw_atmosphere(&self, scene: &Scene) {
        gpu_profile_event!(self.gpu, "atmosphere");

        {
            let mut data = self.data.lock();
            let water_existing = data
                .externs
                .water
                .as_ref()
                .cloned()
                .unwrap_or(ExternDefault::extern_default());

            data.externs.water = Some(externs::Water {
                unk08: data.gbuffers.shading_result_read.view.clone().into(),
                ..water_existing
            });

            let atmos_existing = data
                .externs
                .atmosphere
                .as_ref()
                .cloned()
                .unwrap_or(ExternDefault::extern_default());

            data.externs.atmosphere = Some({
                let atmosphere_lookup_tex_desc = data.gbuffers.atmos_ss_far_lookup.get_desc();
                let atmosphere_lookup_resolution = Vec4::new(
                    atmosphere_lookup_tex_desc.Width as f32,
                    atmosphere_lookup_tex_desc.Height as f32,
                    1. / atmosphere_lookup_tex_desc.Width as f32,
                    1. / atmosphere_lookup_tex_desc.Height as f32,
                );
                externs::Atmosphere {
                    atmos_ss_far_lookup: data.gbuffers.atmos_ss_far_lookup.view.clone().into(),
                    atmos_ss_near_lookup: data.gbuffers.atmos_ss_near_lookup.view.clone().into(),
                    atmosphere_lookup_resolution,
                    unk100: self.gpu.dark_grey_texture.view.clone().into(),
                    light_shaft_optical_depth: self.gpu.white_texture.view.clone().into(),

                    ..atmos_existing
                }
            });
        }

        if scene.get_resource::<MapAtmosphere>().is_some() {
            self.gpu
                .current_states
                .store(StateSelection::new(Some(0), Some(0), Some(0), Some(0)));

            if let Some(atmos) = self.data.lock().externs.atmosphere.as_mut() {
                scene
                    .get_resource::<MapAtmosphere>()
                    .unwrap()
                    .update_extern(atmos, false);
            }

            {
                gpu_profile_event!(self.gpu, "sky_lookup_generate_near");
                self.data.lock().gbuffers.atmos_ss_near_lookup.bind();

                self.execute_global_pipeline(
                    &self.render_globals.pipelines.sky_lookup_generate_near,
                    "sky_lookup_generate_near",
                );
            }

            if let Some(atmos) = self.data.lock().externs.atmosphere.as_mut() {
                scene
                    .get_resource::<MapAtmosphere>()
                    .unwrap()
                    .update_extern(atmos, true);
            }

            {
                gpu_profile_event!(self.gpu, "sky_lookup_generate_far");
                self.data.lock().gbuffers.atmos_ss_far_lookup.bind();

                self.execute_global_pipeline(
                    &self.render_globals.pipelines.sky_lookup_generate_far,
                    "sky_lookup_generate_far",
                );
            }

            {
                gpu_profile_event!(self.gpu, "atmo_depth_angle_density_lookup_generate");
                self.data.lock().gbuffers.depth_angle_density_lookup.bind();

                self.execute_global_pipeline(
                    &self
                        .render_globals
                        .pipelines
                        .atmo_depth_angle_density_lookup_generate,
                    "atmo_depth_angle_density_lookup_generate",
                );
            }
        }
    }
}
