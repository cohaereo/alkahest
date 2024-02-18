use std::time::Instant;

use egui::{ahash::HashMap, Color32, RichText};
use winit::window::Window;

use super::gui::Overlay;
use crate::{
    render::resource_mt::{self, LoadingThreadState},
    resources::Resources,
};

pub struct LoadIndicatorOverlay;

pub const SPINNER_FRAMES: &[&str] = &[
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
pub const SPINNER_INTERVAL: usize = 50;
impl Overlay for LoadIndicatorOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &mut Resources,
        _gui: &mut super::gui::GuiContext<'_>,
    ) -> bool {
        let mut open = *resource_mt::STATUS_TEXTURES.read() != LoadingThreadState::Idle
            || *resource_mt::STATUS_BUFFERS.read() != LoadingThreadState::Idle;
        // || *resource_mt::STATUS_TEXTURES.read() != LoadingThreadState::Idle;

        let indicators = resources.get::<LoadIndicators>();
        open |= indicators.map_or(false, |i| !i.is_empty());

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
                        self.show_indicator(
                            ui,
                            format!("Loading {remaining} textures"),
                            start_time,
                        );
                    }

                    if let LoadingThreadState::Loading {
                        start_time,
                        remaining,
                    } = *resource_mt::STATUS_BUFFERS.read()
                    {
                        self.show_indicator(ui, format!("Loading {remaining} buffers"), start_time);
                    }

                    if let Some(indicators) = resources.get::<LoadIndicators>() {
                        for i in indicators.values() {
                            if i.active {
                                self.show_indicator(ui, &i.label, i.start_time);
                            }
                        }
                    }
                });
        }

        true
    }
}

impl LoadIndicatorOverlay {
    fn show_indicator<L: AsRef<str>>(&self, ui: &mut egui::Ui, label: L, start_time: Instant) {
        let time_millis = start_time.elapsed().as_millis() as usize;
        ui.label(
            RichText::new(format!(
                "{} {} ({:.1}s)",
                SPINNER_FRAMES[(time_millis / SPINNER_INTERVAL) % SPINNER_FRAMES.len()],
                label.as_ref(),
                start_time.elapsed().as_secs_f32()
            ))
            .size(18.0)
            .color(Color32::WHITE),
        );
    }
}

pub type LoadIndicators = HashMap<String, LoadIndicator>;

pub struct LoadIndicator {
    pub start_time: Instant,
    pub label: String,
    pub active: bool,
}

impl LoadIndicator {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            start_time: Instant::now(),
            label: label.into(),
            active: true,
        }
    }

    pub fn restart(&mut self) {
        self.start_time = Instant::now();
        self.active = true;
    }
}
