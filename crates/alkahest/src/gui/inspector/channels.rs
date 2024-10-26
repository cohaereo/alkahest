use alkahest_renderer::{ecs::channels::ObjectChannels, icons::ICON_TABLE};

use crate::gui::UiExt;

use super::ComponentPanel;

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
        _: bevy_ecs::world::EntityRef<'s>,
        ui: &mut egui::Ui,
        _: &alkahest_renderer::resources::AppResources,
    ) {
        for (channel_id, value) in &mut self.values {
            ui.horizontal(|ui| {
                ui.strong(format!("{channel_id:08X}"));
                ui.vec4_input(value);
            });
        }
    }
}
