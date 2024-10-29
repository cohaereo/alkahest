use alkahest_renderer::{
    ecs::channels::ObjectChannels, icons::ICON_TABLE, tfx::channels::ChannelType,
};
use egui::Widget;

use crate::gui::UiExt;

use super::ComponentPanel;
use bevy_ecs::system::Commands;

impl ComponentPanel for ObjectChannels {
    fn inspector_name() -> &'static str {
        "Object Channels"
    }

    fn inspector_icon() -> char {
        ICON_TABLE
    }

    fn should_show_in_inspector(&self) -> bool {
        // Only show this component in the inspector if there are any known channels for this object
        !self.values.is_empty()
    }

    fn show_inspector_ui<'s>(
        &mut self,
        _: &'s mut alkahest_renderer::ecs::Scene,
        _: &mut Commands<'_, '_>,
        _: bevy_ecs::world::EntityRef<'s>,
        ui: &mut egui::Ui,
        _: &alkahest_renderer::resources::AppResources,
    ) {
        for (channel_id, (value, channel_type)) in &mut self.values {
            ui.horizontal(|ui| {
                ui.strong(format!("{channel_id:08X}"));

                match channel_type {
                    alkahest_renderer::tfx::channels::ChannelType::Vec4 => {
                        ui.vec4_input(value);
                    }
                    alkahest_renderer::tfx::channels::ChannelType::Float => {
                        egui::DragValue::new(&mut value.x)
                            .speed(0.01)
                            .ui(ui)
                            .context_menu(|ui| {
                                if ui.selectable_label(false, "Convert to slider").clicked() {
                                    *channel_type = ChannelType::FloatRanged(0.0..=1.0);
                                }
                            });
                    }
                    alkahest_renderer::tfx::channels::ChannelType::FloatRanged(ref range) => {
                        ui.spacing_mut().slider_width = 250.0;
                        egui::Slider::new(&mut value.x, range.clone())
                            .ui(ui)
                            .context_menu(|ui| {
                                if ui
                                    .selectable_label(false, "Convert to drag value")
                                    .clicked()
                                {
                                    *channel_type = ChannelType::Float;
                                }
                            });
                    }
                    alkahest_renderer::tfx::channels::ChannelType::Color => {
                        let mut c = value.truncate().to_array();

                        if ui.color_edit_button_rgb(&mut c).changed() {
                            value.x = c[0];
                            value.y = c[1];
                            value.z = c[2];
                        }
                    }
                }
            });
        }
    }
}
