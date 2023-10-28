use egui::{Color32, RichText};
use winit::window::Window;

use crate::{
    render::resource_mt::{self, LoadingThreadState},
    resources::Resources,
};

use super::gui::Overlay;

pub struct LoadIndicatorOverlay {
    window_rect: egui::Rect,
}

impl Default for LoadIndicatorOverlay {
    fn default() -> Self {
        Self {
            window_rect: egui::Rect::NOTHING,
        }
    }
}

const SPINNER_FRAMES: &[&str] = &[
    "\u{F144B}", // ICON_CLOCK_TIME_ONE_OUTLINE
    "\u{F144C}", // ICON_CLOCK_TIME_TWO_OUTLINE
    "\u{F144D}", // ICON_CLOCK_TIME_THREE_OUTLINE
    "\u{F144E}", // ICON_CLOCK_TIME_FOUR_OUTLINE
    "\u{F144F}", // ICON_CLOCK_TIME_FIVE_OUTLINE
    "\u{F1450}", // ICON_CLOCK_TIME_SIX_OUTLINE
    "\u{F1451}", // ICON_CLOCK_TIME_SEVEN_OUTLINE
    "\u{F1452}", // ICON_CLOCK_TIME_EIGHT_OUTLINE
    "\u{F1453}", // ICON_CLOCK_TIME_NINE_OUTLINE
    "\u{F1454}", // ICON_CLOCK_TIME_TEN_OUTLINE
    "\u{F1455}", // ICON_CLOCK_TIME_ELEVEN_OUTLINE
    "\u{F1456}", // ICON_CLOCK_TIME_TWELVE_OUTLINE
];
const SPINNER_INTERVAL: usize = 50;
impl Overlay for LoadIndicatorOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        _resources: &mut Resources,
        _gui: super::gui::GuiContext<'_>,
    ) -> bool {
        let open = *resource_mt::STATUS_TEXTURES.read() != LoadingThreadState::Idle
            || *resource_mt::STATUS_BUFFERS.read() != LoadingThreadState::Idle;
        // || *resource_mt::STATUS_TEXTURES.read() != LoadingThreadState::Idle;

        if open {
            egui::Window::new("Loading")
                .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
                .title_bar(false)
                .show(ctx, |ui| {
                    if let LoadingThreadState::Loading {
                        start_time,
                        remaining,
                    } = *resource_mt::STATUS_TEXTURES.read()
                    {
                        let time_millis = start_time.elapsed().as_millis() as usize;
                        ui.label(
                            RichText::new(format!(
                                "{} Loading {} textures ({:.1}s)",
                                SPINNER_FRAMES
                                    [(time_millis / SPINNER_INTERVAL) % SPINNER_FRAMES.len()],
                                remaining,
                                start_time.elapsed().as_secs_f32()
                            ))
                            .size(18.0)
                            .color(Color32::WHITE),
                        );
                    }

                    if let LoadingThreadState::Loading {
                        start_time,
                        remaining,
                    } = *resource_mt::STATUS_BUFFERS.read()
                    {
                        let time_millis = start_time.elapsed().as_millis() as usize;
                        ui.label(
                            RichText::new(format!(
                                "{} Loading {} buffers ({:.1}s)",
                                SPINNER_FRAMES
                                    [(time_millis / SPINNER_INTERVAL) % SPINNER_FRAMES.len()],
                                remaining,
                                start_time.elapsed().as_secs_f32()
                            ))
                            .size(18.0)
                            .color(Color32::WHITE),
                        );
                    }

                    self.window_rect = ctx.used_rect();
                });
        }

        true
    }
}
