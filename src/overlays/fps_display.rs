use egui::Color32;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use std::time::Instant;
use winit::window::Window;

use crate::resources::Resources;

use super::gui::Overlay;

pub struct FpsDisplayOverlay {
    pub deltas: ConstGenericRingBuffer<f32, 25>,
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

impl Overlay for FpsDisplayOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        _resources: &mut Resources,
        _gui: super::gui::GuiContext<'_>,
    ) -> bool {
        let average_delta = self.deltas.iter().sum::<f32>() / self.deltas.len() as f32;

        let painter = ctx.layer_painter(egui::LayerId::debug());
        painter.text(
            [ctx.input(|i| i.screen_rect.right()) - 8.0, 8.0].into(),
            egui::Align2::RIGHT_TOP,
            format!("{:3.0}", 1.0 / average_delta),
            egui::FontId::default(),
            Color32::WHITE,
        );

        let now = Instant::now();
        let delta = self.last_frame.elapsed().as_secs_f32();
        self.deltas.push(delta);
        self.last_frame = now;

        true
    }
}
