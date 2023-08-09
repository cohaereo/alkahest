use imgui::WindowFlags;
use winit::window::Window;

use super::gui::OverlayProvider;

pub struct FpsDisplayOverlay {
    pub delta: f32,
}

impl OverlayProvider for FpsDisplayOverlay {
    fn create_overlay(&mut self, ui: &mut imgui::Ui, _window: &Window) {
        ui.window("FPS")
            .flags(WindowFlags::NO_TITLE_BAR | WindowFlags::NO_RESIZE)
            .build(|| ui.text(format!("{:.1}", 1.0 / self.delta)));
    }
}
