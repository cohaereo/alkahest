use alkahest_data::map::SBubbleParent;
use egui::{Margin, Ui};
use tiger_parse::TigerReadable;
use tiger_pkg::{TagHash, package_manager};

use super::{Tab, TabResult, map::MapTab};
use crate::ui::util::UiExt;

pub struct MapListTab {
    map_tags: Vec<TagHash>,
}

impl MapListTab {
    pub fn new() -> Self {
        Self {
            map_tags: package_manager()
                .get_all_by_reference(SBubbleParent::ID.unwrap())
                .into_iter()
                .map(|(t, _)| t)
                .collect(),
        }
    }

    pub fn ui(&self, ui: &mut Ui) -> TabResult {
        let mut result = TabResult::Continue;

        egui::Frame::new()
            .outer_margin(Margin::symmetric(64, 64))
            .show(ui, |ui| {
                for tag in &self.map_tags {
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

        result
    }
}
