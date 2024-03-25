use std::time::Instant;

use egui::{ahash::HashMap, Color32, RichText};
use once_cell::sync::Lazy;
use winit::window::Window;

use super::gui::Overlay;
use crate::{
    icons,
    render::resource_mt::{self, LoadingThreadState},
    resources::Resources,
};

pub struct LoadIndicatorOverlay;

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
        ui.label(
            RichText::new(format!(
                "{} {} ({:.1}s)",
                LoadingIcon::Clock.get_frame(),
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

pub enum LoadingIcon {
    /// A simple, indeterminate spinning clock
    Clock,
    // Indeterminate circle slice animation
    Circle,
}

pub static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);

impl LoadingIcon {
    pub const CLOCK_FRAMES: [char; 12] = [
        icons::ICON_CLOCK_TIME_ONE_OUTLINE,
        icons::ICON_CLOCK_TIME_TWO_OUTLINE,
        icons::ICON_CLOCK_TIME_THREE_OUTLINE,
        icons::ICON_CLOCK_TIME_FOUR_OUTLINE,
        icons::ICON_CLOCK_TIME_FIVE_OUTLINE,
        icons::ICON_CLOCK_TIME_SIX_OUTLINE,
        icons::ICON_CLOCK_TIME_SEVEN_OUTLINE,
        icons::ICON_CLOCK_TIME_EIGHT_OUTLINE,
        icons::ICON_CLOCK_TIME_NINE_OUTLINE,
        icons::ICON_CLOCK_TIME_TEN_OUTLINE,
        icons::ICON_CLOCK_TIME_ELEVEN_OUTLINE,
        icons::ICON_CLOCK_TIME_TWELVE_OUTLINE,
    ];

    pub const CIRCLE_FRAMES: [char; 8] = [
        icons::ICON_CIRCLE_SLICE_1,
        icons::ICON_CIRCLE_SLICE_2,
        icons::ICON_CIRCLE_SLICE_3,
        icons::ICON_CIRCLE_SLICE_4,
        icons::ICON_CIRCLE_SLICE_5,
        icons::ICON_CIRCLE_SLICE_6,
        icons::ICON_CIRCLE_SLICE_7,
        icons::ICON_CIRCLE_SLICE_8,
    ];

    pub const CLOCK_INTERVAL: usize = 50;
    pub const CIRCLE_INTERVAL: usize = 100;

    pub fn get_frame(&self) -> char {
        self.get_frame_with_time(*START_TIME)
    }

    pub fn get_frame_with_time(&self, time: Instant) -> char {
        let time_millis = time.elapsed().as_millis() as usize;
        match self {
            LoadingIcon::Clock => {
                Self::CLOCK_FRAMES[(time_millis / Self::CLOCK_INTERVAL) % Self::CLOCK_FRAMES.len()]
            }
            LoadingIcon::Circle => {
                Self::CIRCLE_FRAMES
                    [(time_millis / Self::CIRCLE_INTERVAL) % Self::CIRCLE_FRAMES.len()]
            }
        }
    }
}
