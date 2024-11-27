use std::collections::HashMap;

use alkahest_renderer::{
    ecs::channels::ObjectChannels, icons::ICON_TABLE, tfx::channels::ChannelType,
};
use bevy_ecs::system::Commands;
use egui::{Color32, RichText, Widget};

use super::ComponentPanel;
use crate::gui::UiExt;

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
        resources: &alkahest_renderer::resources::AppResources,
    ) {
        let wordlist = resources.get::<FnvWordlist>();
        for (channel_id, (value, channel_type)) in &mut self.values {
            ui.horizontal(|ui| {
                if let Some(name) = wordlist.get(*channel_id) {
                    ui.strong(
                        RichText::new(format!("{name} ({channel_id:08X})"))
                            .color(Color32::LIGHT_BLUE),
                    );
                } else {
                    ui.strong(format!("{channel_id:08X}"));
                }

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

pub struct FnvWordlist(HashMap<u32, String>);

impl FnvWordlist {
    /// Initializes a hashmap with the fnv hashes of all the strings in the embedded wordlist
    pub fn new() -> Self {
        let mut map: HashMap<u32, String> = HashMap::new();

        const WORDLIST: &'static str = include_str!("../../../wordlist_channels.txt");
        for s in WORDLIST.lines() {
            let s = s.to_string();
            let h = fnv1(s.as_bytes());
            map.insert(h, s);
        }

        Self(map)
    }

    pub fn get(&self, hash: u32) -> Option<&str> {
        self.0.get(&hash).map(|v| v.as_str())
    }
}

const FNV1_BASE: u32 = 0x811c9dc5;
const FNV1_PRIME: u32 = 0x01000193;
fn fnv1(data: &[u8]) -> u32 {
    data.iter().fold(FNV1_BASE, |acc, b| {
        acc.wrapping_mul(FNV1_PRIME) ^ (*b as u32)
    })
}
