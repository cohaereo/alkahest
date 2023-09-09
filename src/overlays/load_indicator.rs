use imgui::Condition;
use winit::window::Window;

use crate::{
    render::resource_mt::{self, LoadingThreadState},
    resources::Resources,
};

use super::gui::OverlayProvider;

pub struct LoadIndicatorOverlay {
    window_size: [f32; 2],
}

impl Default for LoadIndicatorOverlay {
    fn default() -> Self {
        Self {
            window_size: [0.0; 2],
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
impl OverlayProvider for LoadIndicatorOverlay {
    fn create_overlay(&mut self, ui: &mut imgui::Ui, _window: &Window, _resources: &mut Resources) {
        let open = *resource_mt::STATUS_TEXTURES.read() != LoadingThreadState::Idle
            || *resource_mt::STATUS_TEXTURES.read() != LoadingThreadState::Idle
            || *resource_mt::STATUS_TEXTURES.read() != LoadingThreadState::Idle;

        if open {
            ui.window("Loading")
                .no_inputs()
                .title_bar(false)
                .resizable(false)
                .save_settings(false)
                .position(
                    [ui.io().display_size[0] - self.window_size[0] - 12.0, 12.0],
                    Condition::Always,
                )
                .build(|| {
                    ui.set_window_font_scale(1.1);

                    if let LoadingThreadState::Loading {
                        start_time,
                        remaining,
                    } = *resource_mt::STATUS_TEXTURES.read()
                    {
                        let time_millis = start_time.elapsed().as_millis() as usize;
                        ui.text(format!(
                            "{} Loading {} textures ({:.1}s)",
                            SPINNER_FRAMES[(time_millis / SPINNER_INTERVAL) % SPINNER_FRAMES.len()],
                            remaining,
                            start_time.elapsed().as_secs_f32()
                        ));
                    }

                    self.window_size = ui.window_size();
                });
        }
    }
}
