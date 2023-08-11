use imgui::{Condition, WindowFlags};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use std::time::Instant;
use winit::window::Window;

use super::gui::OverlayProvider;

pub struct FpsDisplayOverlay {
    pub deltas: ConstGenericRingBuffer<f32, 120>,
    last_frame: Instant,
}

impl Default for FpsDisplayOverlay {
    fn default() -> Self {
        Self {
            deltas: Default::default(),
            last_frame: Instant::now(),
        }
    }
}

impl OverlayProvider for FpsDisplayOverlay {
    fn create_overlay(&mut self, ui: &mut imgui::Ui, _window: &Window) {
        let average_delta = self.deltas.iter().sum::<f32>() / self.deltas.len() as f32;
        ui.window("FPS")
            .flags(
                WindowFlags::NO_TITLE_BAR
                    | WindowFlags::NO_RESIZE
                    | WindowFlags::NO_BACKGROUND
                    | WindowFlags::NO_INPUTS,
            )
            .position([ui.io().display_size[0] - 36.0, 0.0], Condition::Always)
            .build(|| ui.text(format!("{:3.0}", 1.0 / average_delta)));

        let now = Instant::now();
        let delta = self.last_frame.elapsed().as_secs_f32();
        self.deltas.push(delta);
        self.last_frame = now;
    }
}
