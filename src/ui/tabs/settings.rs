use egui::{FontId, RichText, TextStyle, Ui};

use crate::{app::SharedState, world::node_filter::NodeFilter};

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

        ui.separator();
        ui.heading("Node Visualization");

        ui.checkbox(&mut config.visual.node_nametags, "Show node nametags");

        ui.add_enabled_ui(config.visual.node_nametags, |ui| {
            ui.checkbox(
                &mut config.visual.node_nametags_named_only,
                "Only show named nodes",
            );

            ui.collapsing("Node filters", |ui| {
                for filter in NodeFilter::ALL {
                    let filter_text = RichText::new(format!("{}", filter)).color(filter.color());

                    let key = filter.to_string();
                    let mut checked = config.visual.node_filters.contains(&key);
                    if ui.checkbox(&mut checked, filter_text).changed() {
                        if checked {
                            config.visual.node_filters.insert(key);
                        } else {
                            config.visual.node_filters.remove(&key);
                        }
                    }
                }
            });
        });
    }
}
