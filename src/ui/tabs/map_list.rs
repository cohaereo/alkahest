use alkahest_data::{
    map::{SBubbleParent, SBubbleParentShallow},
    strings::StringContainerShared,
};
use egui::{Margin, TextEdit, Ui, ahash::HashMap};
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{TagHash, package_manager};

use super::{Tab, TabResult, map::MapTab};
use crate::ui::util::UiExt;

pub struct MapListTab {
    map_tags_by_package: Vec<(String, Vec<(TagHash, String)>)>,
    map_tags_by_package_filtered: Vec<(String, Vec<(TagHash, String)>)>,

    filter: String,
}

impl MapListTab {
    pub fn new(strings: &StringContainerShared) -> Self {
        let map_tags: Vec<(TagHash, String)> = package_manager()
            .get_all_by_reference(SBubbleParent::ID.unwrap())
            .into_iter()
            .filter_map(|(t, _)| {
                let map = package_manager()
                    .read_tag_struct::<SBubbleParentShallow>(t)
                    .ok();
                let map_name_hash = match map {
                    Some(m) => m.map_name,
                    None => return None,
                };
                Some((t, strings.get(map_name_hash)))
            })
            .collect();

        let map_tags_by_package = {
            let mut map = HashMap::default();
            for (tag, name) in map_tags {
                let pkg_path = &package_manager().package_paths[&tag.pkg_id()];
                map.entry(pkg_path.name.clone())
                    .or_insert_with(Vec::new)
                    .push((tag, name));
            }
            let mut vec: Vec<(String, Vec<(TagHash, String)>)> = map.into_iter().collect();
            vec.sort_by(|a, b| a.0.cmp(&b.0));
            vec
        };

        Self {
            map_tags_by_package_filtered: map_tags_by_package.clone(),
            map_tags_by_package,
            filter: String::new(),
        }
    }

    fn filter_tags(&mut self) {
        if self.filter.is_empty() {
            self.map_tags_by_package_filtered = self.map_tags_by_package.clone();
            return;
        }

        self.map_tags_by_package_filtered = self
            .map_tags_by_package
            .iter()
            .map(|(package_name, map_tags)| {
                let filtered_tags: Vec<(TagHash, String)> = map_tags
                    .iter()
                    .filter(|(tag, name)| {
                        let path = &package_manager().package_paths[&tag.pkg_id()];
                        path.name
                            .to_lowercase()
                            .contains(&self.filter.to_lowercase())
                            || name.to_lowercase().contains(&self.filter.to_lowercase())
                    })
                    .cloned()
                    .collect();
                (package_name.clone(), filtered_tags)
            })
            .filter(|(_, tags)| !tags.is_empty())
            .collect();
    }

    pub fn ui(&mut self, ui: &mut Ui) -> TabResult {
        let mut result = TabResult::Continue;
        egui::Frame::new()
            .outer_margin(Margin {
                top: 16,
                bottom: 0,
                left: 16,
                right: 16,
            })
            .show(ui, |ui| {
                if TextEdit::singleline(&mut self.filter)
                    .hint_text("Filter maps...")
                    .show(ui)
                    .response
                    .changed()
                {
                    self.filter_tags();
                }

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (package_name, map_tags) in &self.map_tags_by_package_filtered {
                            ui.separator();
                            ui.heading(package_name);
                            ui.add_space(8.0);
                            ui.horizontal_wrapped(|ui| {
                                for (tag, name) in map_tags.iter() {
                                    let tag = *tag;
                                    let path = &package_manager().package_paths[&tag.pkg_id()];
                                    if ui
                                        .d_button(format!("{} - {name} ({tag})", path.name))
                                        .clicked()
                                    {
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
                        }
                    });
            });

        result
    }
}
