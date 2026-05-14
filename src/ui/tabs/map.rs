use alkahest_data::map::SRespawnPoint;
use alkahest_render::{Renderer, camera::Camera};
use egui::{Color32, Rect, vec2};
use glam::Vec3;
use tiger_pkg::TagHash;

use crate::{
    app::SharedState,
    task::Task,
    ui::{
        scene::{Scene, controller::CameraController},
        util::UiExt,
    },
    world::transform::Transform,
};

pub struct MapTab {
    pub tag: TagHash,
    pub name: String,
    load_task: Task<hecs::World>,
    scene: Box<Scene>,
}

impl MapTab {
    pub fn new(tag: TagHash, name: String, shared: &SharedState) -> anyhow::Result<Self> {
        Ok(Self {
            load_task: Task::new(move || {
                let mut world = hecs::World::new();
                crate::world::map::load_map_into_world(tag, &mut world)
                    .expect("Failed to load map into world");
                world
            }),
            tag,
            name,
            scene: Box::new(
                Scene::new(Renderer::instance().clone(), Camera::default(), shared)?
                    .with_controller(CameraController::new_first_person()),
            ),
        })
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, egui_d3d11: &mut egui_d3d11::D3D11Renderer) {
        if let Some(map) = self.load_task.get() {
            match map {
                Ok(world) => {
                    let mut spawn_candidates = Vec::new();
                    for (_, (transform, respawn_point)) in
                        world.query::<(&Transform, &SRespawnPoint)>().iter()
                    {
                        spawn_candidates.push((
                            transform.translation,
                            transform.rotation,
                            respawn_point.unk20,
                        ));
                    }

                    // If there's any spawn points labeled 'default' (0x2ea8fb98), filter out the rest
                    if spawn_candidates
                        .iter()
                        .find(|(_, _, p)| *p == 0x2ea8fb98)
                        .is_some()
                    {
                        spawn_candidates.retain(|(_, _, p)| *p == 0x2ea8fb98);
                    }

                    if let Some((translation, rotation, hash)) = fastrand::choice(spawn_candidates)
                    {
                        self.scene.camera.position = translation + Vec3::Z * 2.0;
                        self.scene.camera.rotation = rotation;
                        self.scene
                            .controller
                            .set_yaw_pitch(self.scene.camera.get_yaw_pitch());
                        info!(
                            "Spawning camera at {translation:?} {rotation:?} / {:?} (spawn point \
                             0x{hash:X})",
                            self.scene.camera.get_yaw_pitch()
                        );
                    }

                    self.scene.set_world(world);
                }
                Err(_e) => {
                    error!("Failed to load map: unknown error");
                }
            }
        }

        if self.load_task.is_pending() {
            let (_, rect) = ui.allocate_space(ui.available_size());
            ui.painter()
                .rect_filled(rect, 0, Color32::from_rgb(14, 24, 28));
            ui.d_paint_spinner_at(Rect::from_center_size(rect.center(), vec2(64.0, 64.0)));
            ui.painter().text(
                rect.center() + vec2(0.0, 42.0),
                egui::Align2::CENTER_TOP,
                "Loading...",
                egui::FontId::proportional(24.0),
                Color32::GRAY,
            );
        } else {
            self.scene.show(ui, ui.available_size(), egui_d3d11);
        }
    }
}
