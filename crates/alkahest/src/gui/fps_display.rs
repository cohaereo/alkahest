use std::time::Instant;

use egui::Color32;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use winit::window::Window;

use crate::{
    gui::{
        context::{GuiCtx, GuiView, ViewResult},
        util::PainterExt,
    },
    resources::Resources,
};

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

impl GuiView for FpsDisplayOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        _resources: &Resources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        let average_delta = self.deltas.iter().sum::<f32>() / self.deltas.len() as f32;
        let average_fps = 1.0 / average_delta;

        let color = match average_fps {
            fps if fps >= 59.0 => Color32::GREEN,
            fps if fps >= 29.0 => Color32::GOLD,
            _ => Color32::RED,
        };

        let painter = ctx.layer_painter(egui::LayerId::debug());
        painter.text_with_shadow(
            [ctx.input(|i| i.screen_rect.right()) - 20.0, 22.0].into(),
            egui::Align2::RIGHT_TOP,
            format!("{average_fps:3.0}"),
            egui::FontId::proportional(14.0),
            color,
        );

        painter.text_with_shadow(
            [
                ctx.input(|i| i.screen_rect.right()) - 20.0,
                22.0 + 14.0 + 1.0,
            ]
            .into(),
            egui::Align2::RIGHT_TOP,
            format!("{:.1}ms", average_delta * 1000.0),
            egui::FontId::proportional(14.0),
            color,
        );

        let now = Instant::now();
        let delta = self.last_frame.elapsed().as_secs_f32();
        self.deltas.push(delta);
        self.last_frame = now;

        None
    }
}
