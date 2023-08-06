use std::{rc::Rc, cell::RefCell};
use winit::window::Window;

use crate::FpsCamera;

use super::gui::OverlayProvider;

pub struct CameraPositionOverlay {
    pub camera: Rc<RefCell<FpsCamera>>,
    pub show_map_resources: bool,
    pub map_resource_distance: f32
}

impl OverlayProvider for CameraPositionOverlay {
    fn create_overlay(&mut self, ui: &mut imgui::Ui, window: &Window) {
        ui.window("Debug")
            .build(|| {
                ui.text(format!("X: {}", self.camera.as_ref().borrow().position.x));
                ui.text(format!("Y: {}", self.camera.as_ref().borrow().position.y));
                ui.text(format!("Z: {}", self.camera.as_ref().borrow().position.z));
                ui.separator();
                ui.checkbox("Show map resources", &mut self.show_map_resources);
                if self.show_map_resources {
                    ui.slider("Debug distance", 25.0, 4000.0, &mut self.map_resource_distance);
                }
        });
    }
}

