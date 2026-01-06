pub mod controller;
mod surface_viewer;

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

use alkahest_data::tfx::{FeatureRendererSubscription, common::AxisAlignedBBox};
use alkahest_render::{
    Gpu, Renderer, camera::Camera, gpu::command_list::CommandList, renderer::submit::DebugPipeline,
    tfx::view::View,
};
use bitflags::Flags;
use d3d11::{ShaderResourceView, Texture2D, Texture2dDesc, dxgi};
use egui::{
    FontId, RichText, Sense, TextStyle, Ui, UiBuilder, Vec2, Widget, load::SizedTexture, vec2,
};
use glam::Vec3;
use google_material_symbols::GoogleMaterialSymbols;

use crate::{
    ui::{
        scene::controller::CameraController,
        util::{ExternalDataWidgetExt, UiExt},
    },
    world::{
        render_objects::{s_extract_ambient_occlusion, s_extract_render_objects},
        shadowmap::s_render_all_shadowmaps,
    },
};

pub struct Scene {
    pub world: hecs::World,

    renderer: Arc<Renderer>,
    camera: Camera,
    view: View,
    last_frame_time: Instant,
    sun_light_angle: f32,
    pub render_mode: RenderMode,

    pub controller: CameraController,

    surface: d3d11::Texture2D,
    surface_srv: d3d11::ShaderResourceView,

    profiler_results: Option<String>,
    show_surface_viewer: bool,
}

impl Scene {
    pub fn new(renderer: Arc<Renderer>, camera: Camera) -> anyhow::Result<Self> {
        let (surface, surface_srv) = Self::create_surface(&renderer.gpu, (512, 512))?;

        Ok(Self {
            world: hecs::World::new(),
            view: View::new(&renderer.gpu, (128, 128))?,
            renderer,
            camera,
            sun_light_angle: 60f32,
            render_mode: RenderMode::Shaded,
            controller: CameraController::new_orbit(Vec3::ZERO, 2.5),
            surface,
            surface_srv,
            last_frame_time: Instant::now(),
            profiler_results: None,
            show_surface_viewer: false,
        })
    }

    pub fn with_controller(mut self, controller: CameraController) -> Self {
        self.controller = controller;
        self
    }

    fn create_surface(
        gpu: &Gpu,
        resolution: (u32, u32),
    ) -> anyhow::Result<(Texture2D, ShaderResourceView)> {
        let texture = gpu.create_texture2d(
            &Texture2dDesc::builder()
                .width(resolution.0)
                .height(resolution.1)
                .mip_levels(1)
                .format(dxgi::Format::R8g8b8a8Unorm)
                .bind_flags(d3d11::BindFlags::SHADER_RESOURCE)
                .build(),
            None,
        )?;

        let srv = gpu.create_shader_resource_view(&texture, None)?;

        Ok((texture, srv))
    }

    pub fn set_world(&mut self, world: hecs::World) {
        self.world = world;
    }

    pub fn take_world(&mut self) -> hecs::World {
        std::mem::take(&mut self.world)
    }

    pub fn clear(&mut self) {
        self.world.clear();
    }

    pub fn show(&mut self, ui: &mut Ui, size: Vec2, egui_d3d11: &mut egui_d3d11::D3D11Renderer) {
        let now = Instant::now();
        let delta_time = (now - self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        if self.show_surface_viewer {
            egui::SidePanel::right("surface_viewer").show_inside(ui, |ui| {
                self.show_surface_viewer(ui, egui_d3d11);
            });
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let r = ui
                .image(SizedTexture {
                    id: egui_d3d11.textures_mut().allocate_dx_temporary(
                        self.surface_srv.clone(),
                        Some(egui::TextureFilter::Linear),
                        true,
                    ),
                    size,
                })
                .interact(Sense::CLICK | Sense::DRAG | Sense::HOVER);

            if !ui.is_rect_visible(r.rect) {
                return;
            }

            let mut bar_rect = r.rect;
            bar_rect.set_height(32.0);
            ui.painter().rect_filled(
                bar_rect,
                0.0,
                egui::Color32::from_black_alpha(if ui.rect_contains_pointer(bar_rect) {
                    160
                } else {
                    64
                }),
            );
            ui.allocate_new_ui(UiBuilder::new().max_rect(bar_rect), |ui| {
                egui::menu::bar(ui, |ui| {
                    self.show_toolbar(ui);
                })
            });

            let fps_rect = ui.painter_at(r.rect).text(
                r.rect.right_top() + Vec2::new(0.0, 3.0) + Vec2::splat(1.0),
                egui::Align2::RIGHT_TOP,
                format!("{} ", (1. / delta_time).round()),
                egui::FontId::monospace(16.0),
                egui::Color32::BLACK,
            );

            ui.painter_at(r.rect).text(
                r.rect.right_top() + Vec2::new(0.0, 3.0),
                egui::Align2::RIGHT_TOP,
                format!("{} ", (1. / delta_time).round()),
                egui::FontId::monospace(16.0),
                egui::Color32::GREEN,
            );

            ui.scope_builder(egui::UiBuilder::new().max_rect(r.rect), |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    if self.world.is_empty() {
                        ui.label(
                            RichText::new(format!(
                                "{} Scene is empty",
                                GoogleMaterialSymbols::Warning
                            ))
                            .size(16.0),
                        );
                    }

                    if self.renderer.asset_manager.count_loading() > 0 {
                        ui.label(
                            RichText::new(format!(
                                "{} Loading assets... ({} in progress)",
                                GoogleMaterialSymbols::HardDrive,
                                self.renderer.asset_manager.count_loading()
                            ))
                            .size(16.0),
                        );
                    }
                });
            });

            ui.style_mut().spacing.tooltip_width = 4096.0;
            ui.interact(
                fps_rect,
                "frame_counter_profiler_tooltip".into(),
                Sense::hover(),
            )
            .on_hover_ui(|ui| {
                if let Some(profiler_results) = &self.profiler_results {
                    ui.add(
                        egui::Label::new(RichText::new(profiler_results.clone()).monospace())
                            .extend(),
                    );
                } else {
                    ui.weak("Profiler data not available yet.");
                }
            });

            let size_pixels = size * ui.ctx().pixels_per_point();
            let resolution = (size_pixels.x as u32, size_pixels.y as u32);

            self.controller.update(&mut self.camera, ui, &r, delta_time);

            if r.dragged_by(egui::PointerButton::Middle) {
                let delta_adjusted = r.drag_delta() / 4.0;
                self.sun_light_angle += delta_adjusted.x;
                self.sun_light_angle = self.sun_light_angle.rem_euclid(360.0);
            }

            self.render(delta_time, resolution);
        });
    }

    fn show_toolbar(&mut self, ui: &mut Ui) {
        ui.style_mut().spacing.item_spacing = vec2(8.0, 0.0);
        ui.menu_button(GoogleMaterialSymbols::Tune.to_string(), |ui| {
            self.show_settings_ui(ui);
        })
        .response
        .on_hover_text("Scene Settings");

        if ui
            .selectable_label(
                self.show_surface_viewer,
                GoogleMaterialSymbols::ImageSearch.to_string(),
            )
            .clicked()
        {
            self.show_surface_viewer = !self.show_surface_viewer;
        }

        self.render_mode.ui(ui);
        self.view.subscribed_features.show_input(ui);
    }

    fn show_settings_ui(&mut self, ui: &mut Ui) {
        let Self {
            view: View { settings, .. },
            ..
        } = self;

        ui.style_mut()
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(16.0));
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(12.0));
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(16.0));

        ui.strong("Scene Settings");
        ui.checkbox(&mut settings.vertex_ao, "Vertex AO");
        egui::Slider::new(&mut settings.exposure_scale, 0.001..=2.0)
            .logarithmic(true)
            .text("Exposure Scale")
            .show_value(true)
            .ui(ui);

        ui.checkbox(&mut settings.bloom, "Bloom");
        ui.checkbox(&mut settings.volumetrics, "Volumetrics");
        ui.checkbox(&mut settings.shadows, "Shadows");
        ui.checkbox(&mut settings.multithreading, "Multi-threaded Submit");
    }

    pub fn render(&mut self, delta_time: f32, resolution: (u32, u32)) {
        if resolution != self.surface.get_desc().resolution() {
            let (texture, srv) = Self::create_surface(&self.renderer.gpu, resolution)
                .expect("Failed to resize scene surface");
            self.surface = texture;
            self.surface_srv = srv;
        }

        self.camera.aspect_ratio = resolution.0 as f32 / resolution.1 as f32;
        self.controller.update_rotation(&mut self.camera);
        self.camera.update();
        let camera_to_projective = self.camera.projection_matrix(self.camera.aspect_ratio);
        let world_to_camera = self.camera.view_matrix();
        self.view
            .update(world_to_camera, camera_to_projective, resolution);

        let gpu = &self.renderer.gpu;
        let mut cmd = CommandList::from_device_context(gpu, gpu.context().clone());
        let _gpuspan = self.renderer.profiler.scope(&cmd, "Scene::render (total)");
        self.renderer.frame_packet.write().reset();

        let sun_light_direction = Vec3::new(
            self.sun_light_angle.to_radians().cos(),
            self.sun_light_angle.to_radians().sin(),
            0.7,
        )
        .normalize();

        self.renderer
            .externs
            .get_mut()
            .set_global_channel_by_name("sun_light_direction", sun_light_direction.extend(0.0));

        {
            s_extract_ambient_occlusion(&self.world);
            let mut fp = self.renderer.frame_packet.write();
            s_extract_render_objects(&self.world, &mut fp);
        }

        {
            profiling::scope!("prepare");
            let _gpuspan = self.renderer.profiler.scope(&cmd, "prepare");

            // TODO(cohae): Remove the dependency on the world here, shadowmaps should be part of the frame packet
            s_render_all_shadowmaps(&self.world, &mut cmd, &self.renderer);

            cmd.clear_render_target_view(&gpu.acquire_rtv(), &[0.0, 0.0, 0.0, 1.0]);

            {
                profiling::scope!("visibility");
                let _gpuspan = self.renderer.profiler.scope(&cmd, "visibility");
                self.renderer
                    .frame_packet
                    .write()
                    .frame_nodes
                    .retain(|node| {
                        if let Some(render_object) = self
                            .renderer
                            .objects
                            .write()
                            .get_mut(node.render_object_handle.into())
                        {
                            if !self
                                .view
                                .subscribed_features
                                .is_subscribed(render_object.feature_type)
                            {
                                return false;
                            }
                            render_object.visibility_test(&self.camera)
                        } else {
                            true
                        }
                    });

                // self.renderer
                //     .frame_packet
                //     .write()
                //     .frame_nodes
                //     .par_iter_mut()
                //     .for_each(|node| {
                //         let p = self.renderer.objects.data_ptr();
                //         // SAFETY: We have exclusive access to the frame packet and the objects data, and each render object only has one frame node
                //         unsafe {
                //             if let Some(render_object) =
                //                 (*p).get_mut(node.render_object_handle.into())
                //             {
                //                 if !self
                //                     .view
                //                     .subscribed_features
                //                     .is_subscribed(render_object.feature_type)
                //                 {
                //                     node.visible = false;
                //                 } else {
                //                     node.visible = render_object.visibility_test(&self.camera);
                //                 }
                //             }
                //         }
                //     });
                // self.renderer
                //     .frame_packet
                //     .write()
                //     .frame_nodes
                //     .retain(|node| node.visible);
            }

            for node in self.renderer.frame_packet.read().iter_visible() {
                if let Some(render_object) = self
                    .renderer
                    .objects
                    .write()
                    .get_mut(node.render_object_handle.into())
                {
                    render_object.extract_and_prepare(&self.renderer, &*node.data);
                } else if node.render_object_handle.is_valid() {
                    error!("Render object not found: {:?}", node.render_object_handle);
                }
            }

            // Sort nodes by distance
            self.renderer
                .frame_packet
                .write()
                .frame_nodes
                .sort_by(|n1, n2| {
                    n2.distance
                        .partial_cmp(&n1.distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
        }

        self.renderer
            .submit_world(&mut cmd, &self.view, delta_time, self.render_mode.into());
        cmd.copy_resource(
            &self.renderer.surfaces().get(self.view.output).texture,
            &self.surface,
        );

        // let cmd = self.draw_world(delta_time);
        // self.renderer.gpu.submit_command_list(cmd);

        // if self.show_debug_text
        // {
        //     let gpu = &self.renderer.gpu;
        //     let context = gpu.context();

        //     context.rasterizer_set_viewports(&[d3d11::Viewport::builder()
        //         .width(gpu.swapchain_resolution().0 as f32)
        //         .height(gpu.swapchain_resolution().1 as f32)
        //         .build()]);
        //     context.output_merger_set_render_targets(&[Some(gpu.acquire_rtv())], None);
        //     context.output_merger_set_depth_stencil_state(None, 0);
        //     context.rasterizer_set_state(None);
        //     self.renderer.debug_text.lock().draw(&self.renderer.gpu);
        // }

        drop(_gpuspan);
        self.renderer.profiler.end_frame();

        static FRAME_COUNT: AtomicUsize = AtomicUsize::new(0);
        if FRAME_COUNT
            .fetch_add(1, Ordering::Relaxed)
            .is_multiple_of(10)
        {
            self.profiler_results = Some(self.renderer.profiler.get_results_string());
        }
    }

    pub fn output_srv(&self) -> &d3d11::ShaderResourceView {
        &self.surface_srv
    }

    pub fn copy_output_as_texture(&self) -> anyhow::Result<d3d11::ShaderResourceView> {
        let texture = self
            .renderer
            .gpu
            .create_texture2d(&self.surface.get_desc(), None)?;
        self.renderer
            .gpu
            .context()
            .copy_resource(&self.surface, &texture);
        Ok(self
            .renderer
            .gpu
            .create_shader_resource_view(&texture, None)?)
    }

    pub fn focus_on(&mut self, position: Vec3) {
        match &mut self.controller {
            CameraController::Orbit { target, .. } => {
                *target = position;
            }
            CameraController::FirstPerson { .. } => {
                self.camera.position = position;
            }
        }
    }

    pub fn focus_fit_ortho(&mut self, aabb: &AxisAlignedBBox) {
        match &mut self.controller {
            CameraController::Orbit { target, .. } => {
                *target = aabb.center();
                self.camera.max_ortho_width = aabb.extents().length() * 1.2;
            }
            CameraController::FirstPerson { .. } => {}
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    Lookdev,
    Shaded,
    ShadedNoSun,
    // Matcap,

    // Material:
    Albedo,
    Smoothness,
    Metalness,
    AmbientOcclusion,
    Emission,
    EmissionIntensity,
    Transmission,
    IridescenceId,

    // Geometry:
    DepthEdges,
    WorldNormal,

    // Lighting:
    LightDiffuse,
    LightSpecular,
}

impl RenderMode {
    /// Returns true if the render mode UI changed the value
    pub fn ui(&mut self, ui: &mut Ui) -> bool {
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(16.0));

        let mut changed = false;
        egui::ComboBox::from_id_salt("Render Mode")
            .height(400.0)
            .selected_text(format!("{} {:?}", GoogleMaterialSymbols::EvShadow, self))
            .show_ui(ui, |ui| {
                ui.style_mut()
                    .text_styles
                    .insert(TextStyle::Button, FontId::proportional(16.0));
                ui.style_mut().spacing.button_padding = Vec2::new(8.0, 2.0);
                ui.style_mut().spacing.item_spacing = Vec2::ZERO;

                macro_rules! mode {
                    ($ui:ident, $variant:expr, $name:literal) => {
                        if $ui.selectable_label(*self == $variant, $name).clicked() {
                            *self = $variant;
                            changed = true;
                        }
                    };
                }

                mode!(ui, RenderMode::Lookdev, "Lookdev");
                mode!(ui, RenderMode::Shaded, "Shaded");
                mode!(ui, RenderMode::ShadedNoSun, "Shaded (No Sun)");
                // mode!(ui, RenderMode::Matcap, "Matcap");

                ui.section_separator("Material:");
                mode!(ui, RenderMode::Albedo, "Albedo");
                mode!(ui, RenderMode::Smoothness, "Smoothness");
                mode!(ui, RenderMode::Metalness, "Metalness");
                mode!(ui, RenderMode::AmbientOcclusion, "Ambient Occlusion");
                mode!(ui, RenderMode::Emission, "Emission");
                mode!(ui, RenderMode::EmissionIntensity, "Emission Intensity");
                mode!(ui, RenderMode::Transmission, "Transmission");
                mode!(ui, RenderMode::IridescenceId, "Iridescence ID");

                ui.section_separator("Geometry:");
                mode!(ui, RenderMode::DepthEdges, "Depth Edges");
                mode!(ui, RenderMode::WorldNormal, "World Normal");

                ui.section_separator("Lighting:");
                mode!(ui, RenderMode::LightDiffuse, "Diffuse Light");
                mode!(ui, RenderMode::LightSpecular, "Specular Light");
            });

        changed
    }
}

impl From<RenderMode> for Option<DebugPipeline> {
    fn from(val: RenderMode) -> Self {
        match val {
            RenderMode::Lookdev => None,
            RenderMode::Shaded => Some(DebugPipeline::DeferredShading),
            RenderMode::ShadedNoSun => Some(DebugPipeline::DeferredShadingNoSun),
            // RenderMode::Matcap => Some(DebugPipeline::Matcap),
            RenderMode::Albedo => Some(DebugPipeline::Albedo),
            RenderMode::Smoothness => Some(DebugPipeline::Smoothness),
            RenderMode::Metalness => Some(DebugPipeline::Metalness),
            RenderMode::AmbientOcclusion => Some(DebugPipeline::AmbientOcclusion),
            RenderMode::Emission => Some(DebugPipeline::Emission),
            RenderMode::EmissionIntensity => Some(DebugPipeline::EmissionIntensity),
            RenderMode::Transmission => Some(DebugPipeline::Transmission),
            RenderMode::IridescenceId => Some(DebugPipeline::Overcoat),
            RenderMode::DepthEdges => Some(DebugPipeline::DepthEdges),
            RenderMode::WorldNormal => Some(DebugPipeline::WorldNormal),
            RenderMode::LightDiffuse => Some(DebugPipeline::LightDiffuse),
            RenderMode::LightSpecular => Some(DebugPipeline::LightSpecular),
        }
    }
}

impl ExternalDataWidgetExt for FeatureRendererSubscription {
    fn show_input(&mut self, ui: &mut Ui) -> egui::Response {
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(16.0));

        egui::ComboBox::from_id_salt("Feature Renderers")
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .height(400.0)
            .selected_text(format!(
                "{} Enabled Features",
                GoogleMaterialSymbols::CheckBox
            ))
            .show_ui(ui, |ui| {
                ui.style_mut()
                    .text_styles
                    .insert(TextStyle::Button, FontId::proportional(16.0));
                ui.style_mut().spacing.button_padding = Vec2::new(8.0, 2.0);
                ui.style_mut().spacing.item_spacing = Vec2::ZERO;

                let ctrl = ui.input(|i| i.modifiers.ctrl);
                let alt = ui.input(|i| i.modifiers.alt);
                macro_rules! feature {
                    ($ui:ident, $flag:expr, $name:literal) => {
                        if $ui.selectable_label(self.contains($flag), $name).clicked() {
                            if ctrl {
                                self.clear();
                                self.insert($flag);
                            } else if alt {
                                *self = FeatureRendererSubscription::all();
                                self.remove($flag);
                            } else if self.contains($flag) {
                                self.remove($flag);
                            } else {
                                self.insert($flag);
                            }
                        }
                    };
                }

                feature!(
                    ui,
                    FeatureRendererSubscription::CHUNKED_INSTANCE_OBJECTS,
                    "Static Objects"
                );
                feature!(
                    ui,
                    FeatureRendererSubscription::TERRAIN_PATCH,
                    "Terrain Patches"
                );
                feature!(
                    ui,
                    FeatureRendererSubscription::RIGID_OBJECT,
                    "Rigid Objects"
                );
                feature!(
                    ui,
                    FeatureRendererSubscription::SKY_TRANSPARENT,
                    "Sky Transparents"
                );
                feature!(
                    ui,
                    FeatureRendererSubscription::SPEEDTREE_TREES,
                    "Decorators"
                );
                feature!(
                    ui,
                    FeatureRendererSubscription::DYNAMIC_DECALS,
                    "Dynamic Decals"
                );
                feature!(ui, FeatureRendererSubscription::ROAD_DECALS, "Road Decals");
                feature!(ui, FeatureRendererSubscription::WATER, "Water");
                ui.add_enabled_ui(false, |ui| {
                    feature!(ui, FeatureRendererSubscription::LENS_FLARES, "Lens Flares");
                    feature!(ui, FeatureRendererSubscription::PARTICLES, "Particles");
                });

                ui.section_separator("Lighting");
                feature!(ui, FeatureRendererSubscription::CUBEMAPS, "Cubemaps");
                feature!(
                    ui,
                    FeatureRendererSubscription::CHUNKED_LIGHTS,
                    "Chunked Lights"
                );
                feature!(
                    ui,
                    FeatureRendererSubscription::DEFERRED_LIGHTS,
                    "Deferred Lights"
                );
            })
            .response
    }
}
