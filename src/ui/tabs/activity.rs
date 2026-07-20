use std::{
    sync::{Arc, LazyLock},
    time::Instant,
};

use alkahest_data::activity::SActivity;
use alkahest_render::{Renderer, camera::Camera};
use anyhow::Context;
use egui::{Color32, FontId, Rect, TextStyle, Vec2, vec2};
use google_material_symbols::GoogleMaterialSymbols;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use crate::{
    app::SharedState,
    task::Task,
    ui::{
        scene::{Scene, controller::CameraController},
        util::{DButton, UiExt},
    },
    world::map::{
        load_activity_for_map_into_world, load_activity_phase_into_world, load_map_into_world,
    },
};

pub struct ActivityTab {
    maps: Vec<ActivityMap>,
    current_map_index: usize,

    pub tag: TagHash,
    pub name: String,
    scene: Box<Scene>,
}

impl ActivityTab {
    pub fn new(
        state: &Arc<SharedState>,
        tag: TagHash,
        tab_name: String,
        activity_name: &str,
    ) -> anyhow::Result<Self> {
        let activity: SActivity = package_manager()
            .read_tag_struct(tag)
            .context("Failed to read SActivity")?;
        let activity = Arc::new(activity);
        println!("loading activity {activity_name}");

        let mut current_map_index = 0;
        let mut maps = vec![];
        for (index, map) in activity.unk50.iter().enumerate() {
            let is_valid = if let Some(first_ref) = map.map_references.first() {
                first_ref.hash32().is_some()
            } else {
                false
            };
            maps.push(ActivityMap {
                activity: activity.clone(),
                index: maps.len(),
                is_valid,
                name: state.get_string_by_activity(
                    activity_name.trim_end_matches("_ambient"),
                    map.bubble_name,
                ),
                load_task: Task::default(),
                world: None,
                state: ActivityLoadState::Unloaded,
            });

            if current_map_index == index && !is_valid {
                current_map_index += 1;
            }
        }

        if current_map_index >= maps.len() {
            current_map_index = 0;
        }

        if let Some(map) = maps.get_mut(current_map_index) {
            map.start_load();
        }

        Ok(Self {
            current_map_index,
            maps,
            tag,
            name: tab_name,
            scene: Box::new(
                Scene::new(
                    Renderer::instance().clone(),
                    Camera::default(),
                    state,
                    format!("activity_{tag}"),
                )?
                .with_controller(CameraController::new_first_person()),
            ),
        })
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, egui_d3d11: &mut egui_d3d11::D3D11Renderer) {
        egui::SidePanel::left(format!("activity_{}_map_list", self.tag)).show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([true, false])
                .id_salt("map_list_packages")
                .show(ui, |ui| {
                    ui.style_mut().text_styles.insert(
                        TextStyle::Button,
                        FontId::new(20.0, egui::FontFamily::Name("Medium".into())),
                    );
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    for (i, map) in self.maps.iter_mut().enumerate() {
                        let state = map.poll_load();

                        let mut load_sym = state.symbol();
                        if self.current_map_index == i
                            && load_sym as u32 == GoogleMaterialSymbols::CheckCircle as u32
                        {
                            load_sym = GoogleMaterialSymbols::RadioButtonChecked;
                        }
                        if !map.is_valid {
                            load_sym = GoogleMaterialSymbols::Block;
                        }

                        let map_name = if !map.is_valid {
                            format!("{} (invalid)", map.name)
                        } else {
                            map.name.clone()
                        };

                        let btn = if self.current_map_index == i {
                            DButton::new_white((load_sym.to_string(), map_name))
                        } else {
                            DButton::new((load_sym.to_string(), map_name))
                        };

                        ui.add_enabled_ui(map.is_valid, |ui| {
                            if btn
                                .padding(vec2(8.0, 4.0))
                                .min_size(vec2(340.0, 32.0))
                                .ui(ui)
                                .clicked()
                            {
                                self.current_map_index = i;
                                map.start_load();
                            }
                        });
                    }
                });
        });

        let Some(map) = self.maps.get_mut(self.current_map_index) else {
            return;
        };

        match map.poll_load() {
            ActivityLoadState::Unloaded => {}
            ActivityLoadState::Loading => {
                let (_, rect) = ui.allocate_space(ui.available_size_before_wrap());
                ui.painter()
                    .rect_filled(rect, 0, Color32::from_rgb(14, 24, 28));
                ui.d_paint_spinner_at(Rect::from_center_size(rect.center(), Vec2::splat(96.0)));
                ui.painter().text(
                    rect.center() + vec2(0.0, 42.0),
                    egui::Align2::CENTER_TOP,
                    "Loading...",
                    egui::FontId::proportional(24.0),
                    Color32::GRAY,
                );
            }
            ActivityLoadState::Error => {
                let (_, rect) = ui.allocate_space(ui.available_size_before_wrap());
                ui.painter()
                    .rect_filled(rect, 0, Color32::from_rgb(28, 14, 14));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    GoogleMaterialSymbols::Error,
                    egui::FontId::proportional(96.0),
                    Color32::DARK_RED,
                );
                ui.painter().text(
                    rect.center() + vec2(0.0, 48.0),
                    egui::Align2::CENTER_TOP,
                    "Map load failed",
                    egui::FontId::proportional(24.0),
                    Color32::DARK_RED,
                );
                ui.painter().text(
                    rect.center() + vec2(0.0, 82.0),
                    egui::Align2::CENTER_TOP,
                    "See logs for error information",
                    egui::FontId::proportional(16.0),
                    Color32::DARK_RED,
                );
            }
            ActivityLoadState::Loaded => {
                if let Some(world) = map.world.as_mut() {
                    std::mem::swap(world, &mut self.scene.world);
                    self.scene
                        .show(ui, ui.available_size_before_wrap(), egui_d3d11);
                    self.scene
                        .set_id(format!("activity_{}_map_{}", self.tag, map.index));
                    std::mem::swap(&mut self.scene.world, world);
                }
            }
        }
    }
}

struct ActivityMap {
    activity: Arc<SActivity>,
    index: usize,
    is_valid: bool,

    name: String,
    load_task: Task<hecs::World>,
    world: Option<hecs::World>,
    state: ActivityLoadState,
}

impl ActivityMap {
    fn start_load(&mut self) {
        if let ActivityLoadState::Unloaded = self.state {
            self.state = ActivityLoadState::Loading;
            let activity = self.activity.clone();
            let map_index = self.index;
            self.load_task = Task::new("map_load".to_string(), move || {
                let mut world = hecs::World::new();
                let activity_map = &activity.unk50[map_index];
                // TODO(cohae): It's possible to have multiple maps per phase (see Tower), how do we handle showing those?
                let map_hash = activity_map.map_references[0];

                load_map_into_world(map_hash.hash32(), &mut world).expect("Failed to load map");

                if let Err(e) = load_activity_for_map_into_world(
                    activity.ambient_activity,
                    activity_map.bubble_name,
                    &mut world,
                ) {
                    error!("Failed to load ambient activity: {e}");
                }

                for unk in &activity_map.unk18 {
                    if let Err(e) = load_activity_phase_into_world(unk, &mut world) {
                        error!(
                            "Activity phase load for {} failed: {e}",
                            unk.unk_entity_reference.taghash()
                        )
                    }
                }
                world
            });
        }
    }

    #[must_use]
    fn poll_load(&mut self) -> &ActivityLoadState {
        if let Some(res) = self.load_task.get() {
            match res {
                Ok(world) => {
                    self.state = ActivityLoadState::Loaded;
                    self.world = Some(world);
                }
                Err(_err) => {
                    // TODO(cohae): Proper error handling and user popup(?)
                    error!("Activity map load failed");
                    self.state = ActivityLoadState::Error;
                }
            }
        }

        &self.state
    }
}

enum ActivityLoadState {
    Unloaded,
    Loading,
    Loaded,
    Error,
}

impl ActivityLoadState {
    fn symbol(&self) -> GoogleMaterialSymbols {
        match self {
            ActivityLoadState::Unloaded => GoogleMaterialSymbols::Circle,
            ActivityLoadState::Loading => {
                static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);
                let elapsed = Instant::now().duration_since(*START_TIME);
                const SYMBOLS: [GoogleMaterialSymbols; 5] = [
                    GoogleMaterialSymbols::Circle,
                    GoogleMaterialSymbols::ClockLoader20,
                    GoogleMaterialSymbols::ClockLoader40,
                    GoogleMaterialSymbols::ClockLoader60,
                    GoogleMaterialSymbols::ClockLoader80,
                ];

                SYMBOLS[(elapsed.as_millis() / 100) as usize % SYMBOLS.len()]
            }
            ActivityLoadState::Loaded => GoogleMaterialSymbols::CheckCircle,
            ActivityLoadState::Error => GoogleMaterialSymbols::Error,
        }
    }
}
