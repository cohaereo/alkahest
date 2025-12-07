use alkahest_render::{Renderer, camera::Camera};
use egui::{Color32, Rect, vec2};
use tiger_pkg::TagHash;

use crate::{
    task::Task,
    ui::{
        scene::{Scene, controller::CameraController},
        util::spinner_image,
    },
};

pub struct MapTab {
    pub tag: TagHash,
    load_task: Task<hecs::World>,
    scene: Box<Scene>,
}

impl MapTab {
    pub fn new(tag: TagHash) -> anyhow::Result<Self> {
        Ok(Self {
            load_task: Task::new(move || {
                let mut world = hecs::World::new();
                crate::world::map::load_map_into_world(tag, &mut world)
                    .expect("Failed to load map into world");
                world
            }),
            tag,
            scene: Box::new(
                Scene::new(Renderer::instance().clone(), Camera::default())?
                    .with_controller(CameraController::new_first_person()),
            ),
        })
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, egui_d3d11: &mut egui_d3d11::D3D11Renderer) {
        if let Some(map) = self.load_task.get() {
            match map {
                Ok(world) => {
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
                .rect_filled(rect, 0, Color32::from_rgb(45, 48, 56));
            egui::Image::new(spinner_image().clone())
                .paint_at(ui, Rect::from_center_size(rect.center(), vec2(64.0, 64.0)));
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
