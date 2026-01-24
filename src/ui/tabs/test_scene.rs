use alkahest_data::tfx::common::AxisAlignedBBox;
use alkahest_render::{Renderer, camera::Camera};
use egui::{Color32, Rect, vec2};
use glam::{Quat, Vec3, vec3};
use tiger_pkg::TagHash;

use crate::{
    task::Task,
    ui::{
        scene::{Scene, controller::CameraController},
        util::UiExt,
    },
    world::{pattern, transform::Transform},
};

pub struct TestSceneTab {
    scene: Box<Scene>,
}

impl TestSceneTab {
    pub fn new() -> anyhow::Result<Self> {
        let mut scene = Box::new(
            Scene::new(Renderer::instance().clone(), Camera::default())?
                .with_controller(CameraController::new_first_person()),
        );

        for pos in [vec3(30.0, -20.0, 18.0), vec3(34.0, -20.0, 18.0)] {
            pattern::spawn_pattern(
                &mut scene.world,
                TagHash(0x80A717DB),
                None,
                Some(Transform::new(pos, Quat::IDENTITY, vec3(1.0, 0.5, 2.0))),
            )?;
        }

        Ok(Self { scene })
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, egui_d3d11: &mut egui_d3d11::D3D11Renderer) {
        for x in 0..64 {
            for y in 0..36 {
                let bounds = AxisAlignedBBox::from_center_extents(
                    Vec3::new(x as f32, 0.0, y as f32),
                    Vec3::ONE * 0.9,
                );

                let visible = self.scene.camera.is_visible(&bounds);
                if !visible {
                    Renderer::instance()
                        .immediate
                        .lock()
                        .aabb_world(&bounds, if visible { 0x00ff00 } else { 0xff0000 });
                }
            }
        }

        self.scene.show(ui, ui.available_size(), egui_d3d11);
    }
}
