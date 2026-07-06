use std::{str::FromStr, sync::atomic::Ordering};

use alkahest_data::tfx::common::AxisAlignedBBox;
use alkahest_render::{Renderer, camera::Camera, renderer::submit::atmosphere::AtmosphereData};
use egui::{
    Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, TextStyle, Ui, Vec2, Widget,
    scroll_area::ScrollSource, vec2,
};
use glam::{Vec3, Vec4};
use hecs::{Entity, World};
use tiger_pkg::{TagHash, TagHash64, package_manager};

use super::TabResult;
use crate::{
    app::SharedState,
    ui::{
        scene::{
            Scene,
            controller::{CameraController, egui_to_glam_vec2},
        },
        util::UiExt,
    },
    world::{
        object::{self, OPTION_KEY_INVALID, ObjectChannels, PermutationConfig},
        render_objects::{DynamicRenderObject, StaticRenderObject, s_are_all_objects_loaded},
    },
};

pub struct ModelListBase<P: ModelProvider> {
    package_sorting: PackageSorting,
    package_ids: Vec<u16>,

    current_package: u16,
    current_tag: TagHash,
    scene: Scene,
    zoom: f32,

    /// Scene used for rendering thumbnails
    thumbnail_scene: Scene,
    hovered_tag: TagHash,
    hover_vector: Vec2,
    hide_empty: bool,

    filter: String,

    config_tab: EntityConfigTab,
    only_show_used_channels: bool,

    provider: P,
}

impl<P: ModelProvider> ModelListBase<P> {
    pub fn new(provider: P, shared: &SharedState) -> Self {
        let mut thumbnail_scene = Scene::new(
            Renderer::instance().clone(),
            Camera {
                max_ortho_width: 1.0,
                projection: alkahest_render::camera::CameraProjection::Orthographic,
                near: 0.1,
                far: 250.0,
                ..Default::default()
            },
            shared,
            "model_list_thumbnails",
        )
        .unwrap()
        .with_controller(CameraController::new_orbit(Vec3::ZERO, 25.0));

        {
            let view_settings = thumbnail_scene.view.settings_mut();
            view_settings.autoexposure = false;
            view_settings.exposure_scale = 0.75;
            view_settings.bloom = false;
        }

        let mut scene = Scene::new(
            Renderer::instance().clone(),
            Camera::default(),
            shared,
            "model_list_main",
        )
        .unwrap();
        *scene.view.settings_mut() = thumbnail_scene.view.settings().clone();
        scene.view.settings_mut().sun_shadows = true;
        scene.camera.far = 100_000.0;

        let apply_scene_configuration = |scene: &mut Scene| {
            scene.set_global_channel_by_name("global_ambient_intensity", Vec4::splat(5.0));
            Self::init_scene(&mut scene.world);
        };

        apply_scene_configuration(&mut scene);
        scene.set_global_channel_by_name("sky_snapshot_intensity", Vec4::splat(0.09));
        apply_scene_configuration(&mut thumbnail_scene);
        thumbnail_scene.set_global_channel_by_name("sky_snapshot_intensity", Vec4::splat(0.03));

        let package_sorting = PackageSorting::Name;
        let mut package_ids = provider.package_keys().to_vec();
        package_sorting.sort_package_ids(&provider, &mut package_ids);

        Self {
            package_sorting,
            package_ids,
            current_package: 0,
            current_tag: TagHash::NONE,
            zoom: 1.0,
            scene,
            thumbnail_scene,
            hide_empty: true,
            hovered_tag: TagHash::NONE,
            hover_vector: Vec2::ZERO,

            filter: String::new(),

            config_tab: EntityConfigTab::Permutations,
            only_show_used_channels: true,

            provider,
        }
    }

    fn init_scene(world: &mut World) {
        if world.query::<&AtmosphereData>().iter().next().is_some() {
            return;
        }
        let am = &Renderer::instance().asset_manager;
        world.spawn((AtmosphereData {
            atmosphere_lookup_near_0: am.load(TagHash64(0x36F0C0D29A440000)),
            atmosphere_lookup_far_0: am.load(TagHash64(0x36F0C0D29A440000)),
            atmosphere_lookup_near_1: am.load(TagHash64(0x419374B990AB0000)),
            atmosphere_lookup_far_1: am.load(TagHash64(0x419374B990AB0000)),
            atmosphere_lookup_vertical: am.load(TagHash(0x80BD7A1E)),
        },));
    }

    fn render_thumbnails(&mut self, egui_ctx: &egui::Context) {
        let Some(entries) = self.provider.package_mut(self.current_package) else {
            return;
        };

        for entry in entries
            .iter_mut()
            .filter(|e| e.thumbnail.is_none() || e.rerender_needed)
        {
            if let Some(mut world) = entry.thumbnail_world.take() {
                if s_are_all_objects_loaded(&world, Renderer::instance()) {
                    let bb = world
                        .query::<&AxisAlignedBBox>()
                        .iter()
                        .next()
                        .map(|(_, bb)| *bb)
                        .unwrap_or(AxisAlignedBBox::from_center_extents(Vec3::ZERO, Vec3::ONE));

                    Self::init_scene(&mut world);
                    self.thumbnail_scene.set_world(world);
                    self.thumbnail_scene.focus_fit_ortho(&bb);
                    self.thumbnail_scene.render(1.0 / 60.0, (512, 512));
                    match self.thumbnail_scene.copy_output_as_texture() {
                        Ok(o) => {
                            entry.thumbnail = Some(o);
                            entry.rerender_needed = false;
                        }
                        Err(e) => {
                            error!("Failed to render thumbnail for {}: {}", entry.hash, e);
                        }
                    }
                    entry.thumbnail_world = Some(self.thumbnail_scene.take_world());
                    break;
                } else {
                    entry.thumbnail_world = Some(world);
                }
            }
        }
        egui_ctx.request_repaint();
    }

    fn clear_thumbnails(&mut self) {
        let Some(entries) = self.provider.package_mut(self.current_package) else {
            return;
        };

        for entry in entries.iter_mut() {
            entry.thumbnail = None;
        }
    }

    fn render_live_preview(&mut self, hash: TagHash, _hover_vector: Vec2) {
        let Some(entries) = self.provider.package_mut(self.current_package) else {
            return;
        };
        let Some(entry) = entries.iter_mut().find(|e| e.hash == hash) else {
            return;
        };
        if let Some(mut world) = entry.thumbnail_world.take() {
            Self::init_scene(&mut world);

            let bb = world
                .query::<&AxisAlignedBBox>()
                .iter()
                .next()
                .map(|(_, bb)| *bb)
                .unwrap_or(AxisAlignedBBox::from_center_extents(Vec3::ZERO, Vec3::ONE));
            self.thumbnail_scene.set_world(world);
            self.thumbnail_scene.focus_fit_ortho(&bb);
            self.thumbnail_scene.controller.set_yaw_pitch(
                CameraController::DEFAULT_YAW_PITCH
                    + egui_to_glam_vec2(self.hover_vector) * glam::vec2(-15.0, 15.0),
            );
            self.thumbnail_scene.render(1.0 / 60.0, (512, 512));
            entry.thumbnail_world = Some(self.thumbnail_scene.take_world());
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, egui_d3d11: &mut egui_d3d11::D3D11Renderer) -> TabResult {
        self.provider.update();
        if self.provider.package_keys().len() != self.package_ids.len() {
            self.package_ids = self.provider.package_keys().to_vec();
            self.package_sorting
                .sort_package_ids(&self.provider, &mut self.package_ids);
        }

        self.provider.load_package(self.current_package);
        self.render_thumbnails(ui.ctx());

        if self.hovered_tag != TagHash::NONE {
            self.render_live_preview(self.hovered_tag, self.hover_vector);
        }

        ui.separator();
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(16.0));
        ui.style_mut().spacing.button_padding = Vec2::new(8.0, 4.0);

        let (filter_hash, filter_valid) = match TagHash::from_str(&self.filter) {
            Ok(o) => (Some(o), true),
            Err(_) => (None, false),
        };

        egui::SidePanel::left(format!("{}_packages_list", self.provider.name())).show_inside(
            ui,
            |ui| {
                if let Some(status) = self.provider.load_status() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(status);
                    });
                }

                egui::TextEdit::singleline(&mut self.filter)
                    .text_color_opt((!filter_valid).then_some(Color32::RED))
                    .hint_text(
                        RichText::new("Search by Hash (8XXXXXXX)...")
                            .color(Color32::GRAY)
                            .italics(),
                    )
                    .ui(ui);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Sort by:").color(Color32::GRAY));
                    egui::ComboBox::new("sorting_mode", "")
                        .selected_text(format!("{:?}", self.package_sorting))
                        .show_ui(ui, |ui| {
                            ui.style_mut()
                                .text_styles
                                .insert(TextStyle::Button, FontId::proportional(16.0));

                            let mut clicked = ui
                                .selectable_value(
                                    &mut self.package_sorting,
                                    PackageSorting::Id,
                                    "Id",
                                )
                                .clicked();
                            clicked |= ui
                                .selectable_value(
                                    &mut self.package_sorting,
                                    PackageSorting::Name,
                                    "Name",
                                )
                                .clicked();
                            clicked |= ui
                                .selectable_value(
                                    &mut self.package_sorting,
                                    PackageSorting::Count,
                                    "Count",
                                )
                                .clicked();

                            if clicked {
                                (self.package_sorting)
                                    .sort_package_ids(&self.provider, &mut self.package_ids);
                            }
                        });
                });

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        let mut pkg_to_clear: Option<u16> = None;

                        for pkg_id in self.package_ids.iter() {
                            if let Some(hash) = filter_hash
                                && *pkg_id != hash.pkg_id()
                            {
                                continue;
                            }

                            let path = &package_manager().package_paths[pkg_id];
                            if ui
                                .selectable_label(
                                    *pkg_id == self.current_package,
                                    (
                                        RichText::new(format!("{pkg_id:04x} -")).weak().italics(),
                                        path.name.clone(),
                                        RichText::new(format!(
                                            "({})",
                                            self.provider.num_models(*pkg_id)
                                        ))
                                        .weak()
                                        .italics()
                                        .small(),
                                    ),
                                )
                                .clicked()
                            {
                                pkg_to_clear = Some(*pkg_id);
                                self.current_package = *pkg_id;
                            }
                        }

                        if let Some(pkg_id) = pkg_to_clear {
                            self.provider.unload_package(pkg_id);
                        }
                    });
            },
        );

        egui::SidePanel::right(format!("{}_scene", self.current_package))
            .default_width(ui.ctx().content_rect().width() * 0.3)
            .show_inside(ui, |ui| {
                ui.take_available_space();
                let permutations_available = self
                    .scene
                    .world
                    .query::<&PermutationConfig>()
                    .iter()
                    .any(|(_, config)| config.permutation_count > 1 || config.is_configurable());

                let object_channels_available = self
                    .scene
                    .world
                    .query::<&ObjectChannels>()
                    .iter()
                    .any(|(_, channels)| !channels.0.is_empty());

                if permutations_available || object_channels_available {
                    ui.style_mut()
                        .text_styles
                        .insert(TextStyle::Button, FontId::proportional(14.0));
                    ui.style_mut()
                        .text_styles
                        .insert(TextStyle::Body, FontId::proportional(12.0));
                    egui::TopBottomPanel::bottom("entities_scene_configuration")
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                if object_channels_available {
                                    ui.selectable_value(
                                        &mut self.config_tab,
                                        EntityConfigTab::Channels,
                                        "Channels",
                                    );
                                } else {
                                    self.config_tab = EntityConfigTab::Permutations;
                                }

                                if permutations_available {
                                    ui.selectable_value(
                                        &mut self.config_tab,
                                        EntityConfigTab::Permutations,
                                        "Permutations",
                                    );
                                } else {
                                    self.config_tab = EntityConfigTab::Channels;
                                }
                            });

                            match self.config_tab {
                                EntityConfigTab::Permutations => self.show_permutation_editor(ui),
                                EntityConfigTab::Channels => self.show_object_channel_editor(ui),
                            }
                        });
                }

                // if let Some((_, config)) = self
                //     .scene
                //     .world
                //     .query::<&mut PermutationConfig>()
                //     .iter()
                //     .next()
                // {
                //     ui.style_mut()
                //         .text_styles
                //         .insert(TextStyle::Button, FontId::proportional(14.0));
                //     ui.style_mut()
                //         .text_styles
                //         .insert(TextStyle::Body, FontId::proportional(12.0));
                //     egui::TopBottomPanel::bottom("entities_scene_configuration").show_inside(
                //         ui,
                //         |ui| {
                //             ui.add_space(12.0);

                //             config.for_each_key_mut(|key, available_values, current_value| {
                //                 ui.horizontal(|ui| {
                //                     ui.label(permutations::find_kv_name_or_default(key));
                //                     egui::ComboBox::from_id_salt(format!(
                //                         "permutation_combo_{key:X}"
                //                     ))
                //                     .selected_text(permutations::find_kv_name_or_default(
                //                         *current_value,
                //                     ))
                //                     .show_ui(ui, |ui| {
                //                         ui.style_mut()
                //                             .text_styles
                //                             .insert(TextStyle::Button, FontId::proportional(16.0));
                //                         ui.style_mut().spacing.button_padding = Vec2::new(8.0, 2.0);
                //                         ui.style_mut().spacing.item_spacing = Vec2::ZERO;

                //                         for value in available_values {
                //                             if *value == OPTION_KEY_INVALID {
                //                                 continue;
                //                             }
                //                             ui.selectable_value(
                //                                 current_value,
                //                                 *value,
                //                                 permutations::find_kv_name_or_default(*value),
                //                             );
                //                         }
                //                     });
                //                 });
                //             });

                //             if config.calculate_permutation_index().is_none() {
                //                 ui.colored_label(
                //                     Color32::YELLOW,
                //                     "Warning: Current configuration does not map to a valid \
                //                      permutation (hover for details)",
                //                 )
                //                 .on_hover_ui(|ui| {
                //                     ui.style_mut()
                //                         .text_styles
                //                         .insert(TextStyle::Body, FontId::proportional(12.0));
                //                     ui.label(
                //                         "The current combination of permutation keys does not \
                //                          correspond to any valid permutation for this \
                //                          model.\nThis is a bug in Alkahest, and may happen more \
                //                          frequently with models that have a large number of \
                //                          options.",
                //                     );
                //                 });
                //             }
                //         },
                //     );
                // }

                // egui::CentralPanel::default().show_inside(ui, |ui| {
                self.scene.show(ui, ui.available_size(), egui_d3d11);
                // });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.hide_empty, "Hide empty");
                ui.separator();
                egui::Slider::new(&mut self.zoom, 0.1f32..=1.5f32)
                    .text("Zoom")
                    .show_value(true)
                    .step_by(0.1)
                    .clamping(egui::SliderClamping::Always)
                    .ui(ui);
                ui.separator();
                if self.thumbnail_scene.render_mode.ui(ui) {
                    self.clear_thumbnails();
                }
                ui.separator();
                if let Some(entries) = self.provider.package(self.current_package) {
                    let total = entries.len();

                    let with_models = entries.iter().filter(|e| !e.is_empty()).count();

                    ui.label(format!(
                        "Showing {}/{} entities",
                        if self.hide_empty { with_models } else { total },
                        total
                    ));
                }
            });

            let mut load_model = None;
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .scroll_source(ScrollSource::MOUSE_WHEEL | ScrollSource::SCROLL_BAR)
                .show(ui, |ui| {
                    let Some(entries) = self.provider.package_mut(self.current_package) else {
                        ui.label("No package selected");
                        return;
                    };
                    ui.horizontal_wrapped(|ui| {
                        let Self {
                            thumbnail_scene,
                            hovered_tag,
                            hover_vector: card_hover_vector,
                            ..
                        } = self;

                        ui.spacing_mut().item_spacing = vec2(16.0, 16.0);
                        for model in entries {
                            if let Some(hash) = filter_hash
                                && model.hash != hash
                            {
                                continue;
                            }

                            if self.hide_empty && model.is_empty() {
                                continue;
                            }

                            const TAG_BOX_HEIGHT: f32 = 30.0;
                            let (card_rect, card_response) = ui.allocate_exact_size(
                                vec2(256.0, 256.0) * self.zoom + vec2(0.0, TAG_BOX_HEIGHT),
                                Sense::click(),
                            );

                            let card_image_rect =
                                card_rect.with_max_y(card_rect.max.y - TAG_BOX_HEIGHT);
                            let card_painter = ui.painter_at(card_rect);
                            card_painter.rect_filled(
                                card_rect,
                                8.0,
                                ui.visuals().widgets.inactive.bg_fill,
                            );
                            card_painter.rect_filled(
                                card_rect.with_min_y(card_rect.max.y - TAG_BOX_HEIGHT),
                                CornerRadius {
                                    se: 8,
                                    sw: 8,
                                    ..Default::default()
                                },
                                Color32::BLACK,
                            );
                            card_painter.text(
                                card_rect.left_bottom() + vec2(8.0, -5.0),
                                egui::Align2::LEFT_BOTTOM,
                                model.hash.to_string(),
                                FontId::proportional(16.0),
                                ui.visuals().text_color(),
                            );

                            let option_count = model.option_count();
                            let permutation_count = model.permutation_count();
                            if option_count >= 1 || permutation_count >= 2 {
                                card_painter.text(
                                    card_rect.right_bottom() + vec2(-8.0, -5.0),
                                    egui::Align2::RIGHT_BOTTOM,
                                    if self.zoom >= 0.70 {
                                        if option_count == 0 {
                                            format!("{permutation_count} variants")
                                        } else {
                                            format!("{option_count} options")
                                        }
                                    } else {
                                        if option_count == 0 {
                                            permutation_count.to_string()
                                        } else {
                                            option_count.to_string()
                                        }
                                    },
                                    FontId::proportional(16.0),
                                    ui.visuals().text_color().gamma_multiply(0.8),
                                );
                            }

                            if let Some(thumbnail) = model.thumbnail.clone() {
                                let srv = if let Some(hover_pos) = card_response.hover_pos() {
                                    ui.ctx().request_repaint();
                                    *card_hover_vector = (hover_pos - card_image_rect.center())
                                        / (card_image_rect.size() / 2.0);
                                    *card_hover_vector = card_hover_vector
                                        .clamp(Vec2::splat(-1.0), Vec2::splat(1.0));

                                    if *hovered_tag != model.hash {
                                        *hovered_tag = model.hash;
                                        // Use the existing thumbnail until we render the live one next frame
                                        thumbnail.clone()
                                    } else {
                                        thumbnail_scene.output_srv().clone()
                                    }
                                } else {
                                    thumbnail.clone()
                                };

                                let tid = egui_d3d11.textures_mut().allocate_dx_temporary(
                                    srv,
                                    Some(egui::TextureFilter::Linear),
                                    true,
                                );
                                card_painter.image(
                                    tid,
                                    card_image_rect,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            } else {
                                if ui.is_rect_visible(card_response.rect) {
                                    model.rerender_needed = true;
                                }
                                ui.d_paint_spinner_at(Rect::from_center_size(
                                    card_image_rect.center(),
                                    vec2(64.0, 64.0),
                                ));
                            }

                            if card_response.hovered() || model.hash == self.current_tag {
                                let opacity = if model.hash == self.current_tag {
                                    1.0
                                } else {
                                    0.5
                                };
                                card_painter.rect_stroke(
                                    card_rect.shrink(2.0),
                                    8.0,
                                    (
                                        2.0,
                                        ui.visuals()
                                            .widgets
                                            .hovered
                                            .fg_stroke
                                            .color
                                            .gamma_multiply(opacity),
                                    ),
                                    egui::StrokeKind::Outside,
                                );
                            }

                            if card_response.clicked() {
                                self.current_tag = model.hash;

                                self.scene.clear();
                                load_model = Some(model.hash);
                            }

                            card_response.context_menu(|ui| {
                                ui.style_mut()
                                    .text_styles
                                    .insert(TextStyle::Button, FontId::proportional(16.0));
                                if ui.button("Copy tag").clicked() {
                                    ui.ctx().copy_text(model.hash.to_string());
                                    ui.close();
                                }
                            });
                        }
                    });
                });

            if let Some(model_hash) = load_model {
                Self::init_scene(&mut self.scene.world);
                match self.provider.load_model(model_hash, &mut self.scene.world) {
                    Ok(_entity) => {
                        let bb = self
                            .scene
                            .world
                            .query::<&AxisAlignedBBox>()
                            .iter()
                            .fold(Option::<AxisAlignedBBox>::None, |acc, (_, bb)| {
                                if let Some(acc) = acc {
                                    Some(acc.union(bb))
                                } else {
                                    Some(*bb)
                                }
                            })
                            .unwrap_or(AxisAlignedBBox::from_center_extents(Vec3::ZERO, Vec3::ONE));

                        self.scene.focus_on(bb.center());
                    }
                    Err(err) => {
                        error!("Failed to load model: {err}");
                    }
                }
            }
        });

        TabResult::Continue
    }

    fn show_permutation_editor(&mut self, ui: &mut Ui) {
        if let Some((_, config)) = self
            .scene
            .world
            .query::<&mut PermutationConfig>()
            .iter()
            .next()
        {
            let manual_mode = config.permutation_index_override.is_some();
            ui.horizontal(|ui| {
                if config.is_configurable() {
                    if ui
                        .selectable_label(!manual_mode, "Calculate Permutation")
                        .clicked()
                    {
                        config.permutation_index_override = None;
                    }
                } else if config.permutation_index_override.is_none() {
                    config.permutation_index_override = Some(0);
                }

                if ui.selectable_label(manual_mode, "Manual Index").clicked()
                    && config.permutation_index_override.is_none()
                {
                    config.permutation_index_override = Some(0);
                }
            });

            if manual_mode {
                let mut index = config.permutation_index_override.unwrap_or(0);
                ui.horizontal(|ui| {
                    ui.label("Permutation Index:");
                    ui.style_mut().spacing.slider_width = ui.available_width() * 0.7;
                    ui.add(egui::Slider::new(
                        &mut index,
                        0..=config.permutation_count.saturating_sub(1),
                    ));
                });

                // Only update when overridden, otherwise we can't switch back to calculation mode
                if config.permutation_index_override.is_some() {
                    config.permutation_index_override = Some(index);
                }
            } else {
                config.for_each_key_mut(|key, available_values, current_value| {
                    ui.horizontal(|ui| {
                        ui.label(object::find_fnv_name_or_default(key));
                        egui::ComboBox::from_id_salt(format!("permutation_combo_{key:X}"))
                            .selected_text(object::find_fnv_name_or_default(*current_value))
                            .show_ui(ui, |ui| {
                                ui.style_mut()
                                    .text_styles
                                    .insert(TextStyle::Button, FontId::proportional(16.0));
                                ui.style_mut().spacing.button_padding = Vec2::new(8.0, 2.0);
                                ui.style_mut().spacing.item_spacing = Vec2::ZERO;

                                for value in available_values {
                                    if *value == OPTION_KEY_INVALID {
                                        continue;
                                    }
                                    ui.selectable_value(
                                        current_value,
                                        *value,
                                        object::find_fnv_name_or_default(*value),
                                    );
                                }
                            });
                    });
                });
            }

            if let Some(permutation_index) = config.calculate_permutation_index() {
                ui.label(format!("Selected permutation: {permutation_index}"));
            } else {
                ui.colored_label(
                    Color32::YELLOW,
                    "Warning: Current configuration does not map to a valid permutation (hover \
                     for details)",
                )
                .on_hover_ui(|ui| {
                    ui.style_mut()
                        .text_styles
                        .insert(TextStyle::Body, FontId::proportional(12.0));
                    ui.label(
                        "The current combination of permutation keys does not correspond to any \
                         valid permutation for this model.\nThis is a bug in Deimos, and may \
                         happen more frequently with models that have a large number of options.",
                    );
                });
            }
        }
    }
    fn show_object_channel_editor(&mut self, ui: &mut Ui) {
        ui.checkbox(&mut self.only_show_used_channels, "Only show used");
        if let Some((_, channels)) = self
            .scene
            .world
            .query::<&mut ObjectChannels>()
            .iter()
            .next()
        {
            ui.spacing_mut().scroll.floating = false;
            let num_channels = if self.only_show_used_channels {
                channels
                    .0
                    .iter()
                    .filter(|c| c.usage.load(Ordering::Relaxed) > 0)
                    .count()
            } else {
                channels.0.len()
            };
            let mut show_inner = |ui: &mut Ui| {
                for channel in channels.0.iter_mut().filter(|c| {
                    if self.only_show_used_channels {
                        c.usage.load(Ordering::Relaxed) > 0
                    } else {
                        true
                    }
                }) {
                    ui.horizontal(|ui| {
                        if let Some(name) = object::find_fnv_name(channel.name) {
                            ui.label(format!("{name} (0x{:08X})", channel.name));
                        } else {
                            ui.label(format!("unk_{:08X}", channel.name));
                        }
                        ui.horizontal(|ui| {
                            ui.spacing_mut().button_padding = vec2(4.0, 1.0);
                            ui.spacing_mut().interact_size = egui::vec2(100.0, 32.0);
                            egui::DragValue::new(&mut channel.value.x)
                                .fixed_decimals(4)
                                .speed(0.01)
                                .ui(ui);
                            egui::DragValue::new(&mut channel.value.y)
                                .fixed_decimals(4)
                                .speed(0.01)
                                .ui(ui);
                            egui::DragValue::new(&mut channel.value.z)
                                .fixed_decimals(4)
                                .speed(0.01)
                                .ui(ui);
                            egui::DragValue::new(&mut channel.value.w)
                                .fixed_decimals(4)
                                .speed(0.01)
                                .ui(ui);
                        });
                    });
                }
            };

            if num_channels <= 4 {
                show_inner(ui);
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    show_inner(ui);
                });
            }
        }
    }
}

pub trait ModelProvider {
    /// Used for widget IDs, must be unique between providers.
    fn name(&self) -> &str;

    fn update(&mut self) {}

    fn load_status(&self) -> Option<String> {
        None
    }

    fn package_keys(&self) -> &[u16];
    fn package(&self, pkg_id: u16) -> Option<&[ModelEntry]>;
    fn package_mut(&mut self, pkg_id: u16) -> Option<&mut [ModelEntry]>;

    fn num_models(&self, pkg_id: u16) -> usize;

    fn load_model(&mut self, hash: TagHash, world: &mut hecs::World) -> anyhow::Result<Entity>;
    fn load_package(&mut self, pkg_id: u16);
    fn unload_package(&mut self, pkg_id: u16);
}

pub struct ModelEntry {
    pub hash: TagHash,
    pub thumbnail_world: Option<hecs::World>,
    pub thumbnail: Option<d3d11::ShaderResourceView>,
    pub rerender_needed: bool,
}

impl ModelEntry {
    fn option_count(&self) -> usize {
        let Some(world) = &self.thumbnail_world else {
            return 0;
        };

        world
            .query::<&PermutationConfig>()
            .iter()
            .map(|(_, config)| config.iter_keys().count())
            .sum()
    }

    fn permutation_count(&self) -> usize {
        let Some(world) = &self.thumbnail_world else {
            return 0;
        };

        world
            .query::<&PermutationConfig>()
            .iter()
            .map(|(_, config)| config.permutation_count)
            .max()
            .unwrap_or(0)
    }

    fn is_empty(&self) -> bool {
        let Some(world) = &self.thumbnail_world else {
            return true;
        };

        world.query::<&DynamicRenderObject>().iter().len() == 0
            && world.query::<&StaticRenderObject>().iter().len() == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageSorting {
    Id,
    Name,
    Count,
}

impl PackageSorting {
    fn sort_package_ids<P: ModelProvider>(&self, provider: &P, packages: &mut [u16]) {
        match self {
            PackageSorting::Id => packages.sort_by_key(|id| *id),
            PackageSorting::Name => packages
                .sort_by_cached_key(|id| (package_manager().package_paths[id].name.clone(), *id)),
            PackageSorting::Count => {
                packages.sort_by_cached_key(|id| provider.num_models(*id));
                packages.reverse();
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityConfigTab {
    Permutations,
    Channels,
}
