use alkahest_data::map::SBubbleParent;
use egui::{Margin, TextEdit, Ui};
use tiger_parse::TigerReadable;
use tiger_pkg::{package_manager, TagHash};

use super::{map::MapTab, Tab, TabResult};
use crate::ui::util::UiExt;

pub struct MapListTab {
    map_tags: Vec<TagHash>,
    filter: String,
}

impl MapListTab {
    pub fn new() -> Self {
        Self {
            map_tags: package_manager()
                .get_all_by_reference(SBubbleParent::ID.unwrap())
                .into_iter()
                .map(|(t, _)| t)
                .collect(),
            filter: String::new(),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> TabResult {
        let mut result = TabResult::Continue;

        egui::Frame::new()
            .outer_margin(Margin::symmetric(64, 64))
            .show(ui, |ui| {
                TextEdit::singleline(&mut self.filter)
                    .hint_text("Filter maps...")
                    .show(ui);
                ui.horizontal_wrapped(|ui| {
                    for tag in self.map_tags.iter().filter(|tag| {
                        let path = &package_manager().package_paths[&tag.pkg_id()];
                        path.name
                            .to_lowercase()
                            .contains(&self.filter.to_lowercase())
                    }) {
                        let tag = *tag;
                        let path = &package_manager().package_paths[&tag.pkg_id()];
                        if ui.d_button(format!("{} - {}", path.name, tag)).clicked() {
                            match MapTab::new(tag) {
                                Ok(map) => {
                                    result = TabResult::Open(Tab::Map(map));
                                }
                                Err(e) => {
                                    error!("Failed to open map tab: {e}");
                                }
                            }
                        }
                    }
                });
            });

        result
    }
}
