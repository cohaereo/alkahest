use std::{
    sync::{Arc, LazyLock},
    time::Instant,
};

use alkahest_data::activity::SActivity;
use alkahest_render::{Renderer, camera::Camera};
use anyhow::Context;
use egui::vec2;
use google_material_symbols::GoogleMaterialSymbols;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use crate::{
    app::SharedState,
    task::Task,
    ui::{
        scene::{Scene, controller::CameraController},
        util::DButton,
    },
    world::map::{
        load_activity_for_map_into_world, load_activity_phase_into_world, load_map_into_world,
    },
};

pub struct ActivityTab {
    state: Arc<SharedState>,
    activity: Arc<SActivity>,
    maps: Vec<ActivityMap>,
    current_map_index: usize,

    pub tag: TagHash,
    pub name: String,
    // load_task: Task<hecs::World>,
    // scene: Box<Scene>,
}

impl ActivityTab {
    pub fn new(state: &Arc<SharedState>, tag: TagHash, name: String) -> anyhow::Result<Self> {
        let activity: SActivity = package_manager()
            .read_tag_struct(tag)
            .context("Failed to read SActivity")?;
        let activity = Arc::new(activity);

        let mut maps = vec![];
        for map in &activity.unk50 {
            maps.push(ActivityMap {
                activity: activity.clone(),
                index: maps.len(),
                name: state.get_string_by_package(
                    &package_manager().package_paths[&tag.pkg_id()].name,
                    map.bubble_name,
                ),
                load_task: Task::default(),
                scene: Box::new(
                    Scene::new(Renderer::instance().clone(), Camera::default())?
                        .with_controller(CameraController::new_first_person()),
                ),
                state: ActivityLoadState::Unloaded,
            });
        }
        if let Some(map) = maps.first_mut() {
            map.start_load();
        }

        Ok(Self {
            state: state.clone(),
            activity,
            current_map_index: 0,
            maps,
            // load_task: Task::new(move || {
            //     let mut world = hecs::World::new();
            //     crate::world::map::load_map_into_world(tag, &mut world)
            //         .expect("Failed to load map into world");
            //     world
            // }),
            tag,
            name,
            // scene: Box::new(
            //     Scene::new(Renderer::instance().clone(), Camera::default())?
            //         .with_controller(CameraController::new_first_person()),
            // ),
        })
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, egui_d3d11: &mut egui_d3d11::D3D11Renderer) {
        egui::SidePanel::left(format!("activity_{}_map_list", self.tag)).show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([true, false])
                .id_salt("map_list_packages")
                .show(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    for (i, map) in self.maps.iter_mut().enumerate() {
                        let state = map.poll_load();

                        let mut load_sym = state.symbol();
                        if self.current_map_index == i
                            && load_sym as u32 == GoogleMaterialSymbols::CheckCircle as u32
                        {
                            load_sym = GoogleMaterialSymbols::RadioButtonChecked;
                        }

                        let btn = if self.current_map_index == i {
                            DButton::new_white((load_sym.to_string(), map.name.clone()))
                        } else {
                            DButton::new((load_sym.to_string(), map.name.clone()))
                        };

                        if btn.min_size(vec2(384.0, 32.0)).ui(ui).clicked() {
                            self.current_map_index = i;
                            map.start_load();
                        }
                    }
                });
        });

        let Some(map) = self.maps.get_mut(self.current_map_index) else {
            return;
        };

        map.scene
            .show(ui, ui.available_size_before_wrap(), egui_d3d11);
    }
}

struct ActivityMap {
    activity: Arc<SActivity>,
    index: usize,

    name: String,
    load_task: Task<hecs::World>,
    state: ActivityLoadState,
    scene: Box<Scene>,
}

impl ActivityMap {
    fn start_load(&mut self) {
        if let ActivityLoadState::Unloaded = self.state {
            self.state = ActivityLoadState::Loading;
            let activity = self.activity.clone();
            let map_index = self.index;
            self.load_task = Task::new(move || {
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
                    self.scene.world = world;
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
