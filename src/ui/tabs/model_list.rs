use alkahest_data::tfx::common::AxisAlignedBBox;
use alkahest_render::{Renderer, camera::Camera};
use egui::{
    Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, TextStyle, Ui, Vec2, Widget,
    scroll_area::ScrollSource, vec2,
};
use glam::{Vec3, Vec4};
use hecs::Entity;
use tiger_pkg::{TagHash, package_manager};

use super::TabResult;
use crate::{
    ui::{
        scene::{
            Scene,
            controller::{CameraController, egui_to_glam_vec2},
        },
        util::UiExt,
    },
    world::{
        permutations::{self, OPTION_KEY_INVALID, PermutationConfig},
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

    provider: P,
}

impl<P: ModelProvider> ModelListBase<P> {
    pub fn new(provider: P) -> Self {
        let mut thumbnail_scene = Scene::new(
            Renderer::instance().clone(),
            Camera {
                max_ortho_width: 1.0,
                projection: alkahest_render::camera::CameraProjection::Orthographic,
                near: 0.1,
                far: 250.0,
                ..Default::default()
            },
        )
        .unwrap()
        .with_controller(CameraController::new_orbit(Vec3::ZERO, 25.0));

        thumbnail_scene.view.settings.autoexposure = false;
        thumbnail_scene.view.settings.exposure_scale = 0.250;
        thumbnail_scene.view.settings.bloom = false;

        let mut scene = Scene::new(Renderer::instance().clone(), Camera::default()).unwrap();
        scene.view.settings = thumbnail_scene.view.settings.clone();
        scene.camera.far = 100_000.0;

        let apply_scene_configuration = |scene: &mut Scene| {
            scene.set_global_channel_by_name("global_ambient_intensity", Vec4::splat(5.0));
        };

        apply_scene_configuration(&mut scene);
        apply_scene_configuration(&mut thumbnail_scene);

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

            provider,
        }
    }

    fn render_thumbnails(&mut self, egui_ctx: &egui::Context) {
        let Some(entries) = self.provider.package_mut(self.current_package) else {
            return;
        };

        for entry in entries.iter_mut().filter(|e| e.thumbnail.is_none()) {
            if let Some(world) = entry.thumbnail_world.take() {
                if s_are_all_objects_loaded(&world, Renderer::instance()) {
                    let bb = world
                        .query::<&AxisAlignedBBox>()
                        .iter()
                        .next()
                        .map(|(_, bb)| *bb)
                        .unwrap_or(AxisAlignedBBox::from_center_extents(Vec3::ZERO, Vec3::ONE));

                    self.thumbnail_scene.set_world(world);
                    self.thumbnail_scene.focus_fit_ortho(&bb);
                    self.thumbnail_scene.render(1.0 / 60.0, (512, 512));
                    match self.thumbnail_scene.copy_output_as_texture() {
                        Ok(o) => {
                            entry.thumbnail = Some(o);
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
        if let Some(world) = entry.thumbnail_world.take() {
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

        egui::SidePanel::left(format!("{}_packages_list", self.provider.name())).show_inside(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Sort by:");
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

                if let Some((_, config)) = self
                    .scene
                    .world
                    .query::<&mut PermutationConfig>()
                    .iter()
                    .next()
                {
                    ui.style_mut()
                        .text_styles
                        .insert(TextStyle::Button, FontId::proportional(14.0));
                    ui.style_mut()
                        .text_styles
                        .insert(TextStyle::Body, FontId::proportional(12.0));
                    egui::TopBottomPanel::bottom("entities_scene_configuration").show_inside(
                        ui,
                        |ui| {
                            ui.add_space(12.0);

                            config.for_each_key_mut(|key, available_values, current_value| {
                                ui.horizontal(|ui| {
                                    ui.label(permutations::find_kv_name_or_default(key));
                                    egui::ComboBox::from_id_salt(format!(
                                        "permutation_combo_{key:X}"
                                    ))
                                    .selected_text(permutations::find_kv_name_or_default(
                                        *current_value,
                                    ))
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
                                                permutations::find_kv_name_or_default(*value),
                                            );
                                        }
                                    });
                                });
                            });

                            if config.calculate_permutation_index().is_none() {
                                ui.colored_label(
                                    Color32::YELLOW,
                                    "Warning: Current configuration does not map to a valid \
                                     permutation (hover for details)",
                                )
                                .on_hover_ui(|ui| {
                                    ui.style_mut()
                                        .text_styles
                                        .insert(TextStyle::Body, FontId::proportional(12.0));
                                    ui.label(
                                        "The current combination of permutation keys does not \
                                         correspond to any valid permutation for this \
                                         model.\nThis is a bug in Alkahest, and may happen more \
                                         frequently with models that have a large number of \
                                         options.",
                                    );
                                });
                            }
                        },
                    );
                }

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
                    let Some(entries) = self.provider.package(self.current_package) else {
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
                            if option_count >= 1 {
                                card_painter.text(
                                    card_rect.right_bottom() + vec2(-8.0, -5.0),
                                    egui::Align2::RIGHT_BOTTOM,
                                    if self.zoom >= 0.70 {
                                        format!("{option_count} options")
                                    } else {
                                        option_count.to_string()
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
                                if ui.button("Copy hash").clicked() {
                                    ui.ctx().copy_text(model.hash.to_string());
                                    ui.close();
                                }

                                if ui.button("Copy hash (Charm)").clicked() {
                                    ui.ctx()
                                        .copy_text(format!("{:08X}", model.hash.0.swap_bytes()));
                                    ui.close();
                                }
                            });
                        }
                    });
                });

            if let Some(model_hash) = load_model {
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
}

pub trait ModelProvider {
    /// Used for widget IDs, must be unique between providers.
    fn name(&self) -> &str;

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
