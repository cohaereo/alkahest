use alkahest_data::{geometry::EPrimitiveType, technique::StateSelection};

use crate::{
    ecs::{map::MapAtmosphere, render::light::draw_light_system, Scene},
    gpu_event,
    renderer::{cubemaps::draw_cubemap_system, Renderer},
    tfx::{
        externs,
        externs::{ExternDefault, ShadowMask},
    },
};

impl Renderer {
    pub fn draw_lighting_pass(&self, scene: &Scene) {
        gpu_event!(self.gpu, "lighting_pass");

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
            if self.render_settings.matcap {
                gpu_event!(self.gpu, "matcap");
                self.matcap.draw(self);
            } else {
                {
                    gpu_event!(self.gpu, "deferred_lights");
                    draw_light_system(self, scene)
                }

                if self.render_settings.feature_cubemaps {
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

                if self.render_settings.feature_global_lighting {
                    gpu_event!(self.gpu, "global_lighting");

                    let pipeline = &self.render_globals.pipelines.global_lighting;
                    if let Err(e) = pipeline.bind(self) {
                        error!("Failed to run global_lighting: {e}");
                        return;
                    }

                    // TODO(cohae): Try to reduce the boilerplate for screen space pipelines like this one
                    self.gpu.current_states.store(StateSelection::new(
                        Some(8),
                        Some(0),
                        Some(0),
                        Some(0),
                    ));
                    self.gpu.flush_states();
                    self.gpu.set_input_topology(EPrimitiveType::TriangleStrip);

                    // TODO(cohae): 4 vertices doesn't work...
                    unsafe {
                        self.gpu.context().Draw(6, 0);
                    }
                }
            }
        }

        {
            if self.render_settings.ssao {
                gpu_event!(self.gpu, "ssao");
                self.ssao.draw(self);
            }
        }
    }

    // TODO(cohae): woe, naming conventions be upon ye
    pub fn draw_shading_pass(&self, scene: &Scene) {
        gpu_event!(self.gpu, "shading_pass");

        unsafe {
            let gbuffers = &self.data.lock().gbuffers;
            self.gpu.context().OMSetRenderTargets(
                Some(&[Some(gbuffers.shading_result.render_target.clone()), None]),
                None,
            );
        }

        unsafe {
            gpu_event!(self.gpu, "deferred_shading");
            let pipeline = if scene.query::<&MapAtmosphere>().iter().next().is_some()
                && self.render_settings.feature_atmosphere
            {
                &self.render_globals.pipelines.deferred_shading
            } else {
                &self.render_globals.pipelines.deferred_shading_no_atm
            };
            if let Err(e) = pipeline.bind(self) {
                error!("Failed to run deferred_shading: {e}");
                return;
            }

            // TODO(cohae): Try to reduce the boilerplate for screen space pipelines like this one
            self.gpu
                .current_states
                .store(StateSelection::new(Some(0), Some(0), Some(0), Some(0)));
            self.gpu.flush_states();
            self.gpu.set_input_topology(EPrimitiveType::TriangleStrip);

            // TODO(cohae): 4 vertices doesn't work...
            self.gpu.context().Draw(6, 0);
        }
    }

    pub fn draw_atmosphere(&self, scene: &Scene) {
        gpu_event!(self.gpu, "atmosphere");

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
                let mut atmos = externs::Atmosphere {
                    atmos_ss_far_lookup: data.gbuffers.atmos_ss_far_lookup.view.clone().into(),
                    atmos_ss_near_lookup: data.gbuffers.atmos_ss_near_lookup.view.clone().into(),
                    unke0: self.gpu.dark_grey_texture.view.clone().into(),

                    ..atmos_existing
                };

                if let Some((_, map_atmos)) = scene.query::<&MapAtmosphere>().iter().next() {
                    map_atmos.update_extern(&mut atmos);
                }

                atmos
            });
        }

        if scene.query::<&MapAtmosphere>().iter().next().is_some() {
            unsafe {
                self.gpu.context().OMSetRenderTargets(
                    Some(&[
                        Some(
                            self.data
                                .lock()
                                .gbuffers
                                .atmos_ss_far_lookup
                                .render_target
                                .clone(),
                        ),
                        None,
                    ]),
                    None,
                );

                self.render_globals
                    .pipelines
                    .sky_lookup_generate_far
                    .bind(self)
                    .unwrap();

                self.gpu.current_states.store(StateSelection::new(
                    Some(0),
                    Some(0),
                    Some(0),
                    Some(0),
                ));
                self.gpu.flush_states();
                self.gpu.set_input_topology(EPrimitiveType::TriangleStrip);

                self.gpu.context().Draw(6, 0);

                self.gpu.context().OMSetRenderTargets(
                    Some(&[
                        Some(
                            self.data
                                .lock()
                                .gbuffers
                                .atmos_ss_near_lookup
                                .render_target
                                .clone(),
                        ),
                        None,
                    ]),
                    None,
                );

                self.render_globals
                    .pipelines
                    .sky_lookup_generate_near
                    .bind(self)
                    .unwrap();

                self.gpu.context().Draw(6, 0);
            }
        }
    }
}
