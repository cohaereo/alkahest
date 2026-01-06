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
    }
}
