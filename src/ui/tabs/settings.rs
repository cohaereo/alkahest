use egui::{FontId, TextStyle, Ui};

use crate::app::SharedState;

pub struct SettingsTab;

impl SettingsTab {
    pub fn ui(ui: &mut Ui, state: &SharedState) {
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(16.0));

        let mut config = state.config.write();
        ui.checkbox(&mut config.vsync, "Enable Vsync");

        ui.checkbox(&mut config.framelimiter_enabled, "Enable Framelimiter");
        ui.spacing_mut().slider_width = 384.0;

        if config.framelimiter_enabled {
            ui.add(
                egui::Slider::new(&mut config.framerate_limit, 20..=240)
                    .step_by(10.0)
                    .text("Framerate Limit")
                    .custom_formatter(|value, _| format!("{} FPS", value)),
            );
        }

        ui.add(
            egui::Slider::new(&mut config.resolution_scale, 0.25..=2.0)
                .step_by(0.25)
                .text("Resolution Scale")
                .custom_formatter(|value, _| format!("{:.0}%", value * 100.0)),
        );
    }
}
