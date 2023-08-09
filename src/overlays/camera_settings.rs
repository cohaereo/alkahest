use std::{cell::RefCell, rc::Rc};
use winit::window::Window;

use crate::FpsCamera;

use super::gui::OverlayProvider;

pub struct CameraPositionOverlay {
    pub camera: Rc<RefCell<FpsCamera>>,
    pub show_map_resources: bool,
    pub show_unknown_map_resources: bool,
    pub map_resource_distance: f32,

    pub render_scale: f32,
    pub render_scale_changed: bool,
}

impl OverlayProvider for CameraPositionOverlay {
    fn create_overlay(&mut self, ui: &mut imgui::Ui, _window: &Window) {
        ui.window("Debug").build(|| {
            ui.text(format!("X: {}", self.camera.as_ref().borrow().position.x));
            ui.text(format!("Y: {}", self.camera.as_ref().borrow().position.y));
            ui.text(format!("Z: {}", self.camera.as_ref().borrow().position.z));
            ui.separator();
            self.render_scale_changed =
                ui.slider("Render Scale", 50.0, 200.0, &mut self.render_scale);
            ui.checkbox("Show map resources", &mut self.show_map_resources);
            if self.show_map_resources {
                ui.checkbox(
                    "Show unknown resources",
                    &mut self.show_unknown_map_resources,
                );
                ui.slider(
                    "Debug distance",
                    25.0,
                    4000.0,
                    &mut self.map_resource_distance,
                );
            }
        });
    }
}
