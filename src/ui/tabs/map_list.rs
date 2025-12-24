use alkahest_data::{
    map::{SBubbleParent, SBubbleParentShallow},
    strings::StringContainerShared,
};
use egui::{Margin, Ui, ahash::HashMap, vec2};
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{TagHash, package_manager};

use super::{Tab, TabResult, map::MapTab};
use crate::ui::util::DButton;

pub struct MapListTab {
    map_tags_by_package: Vec<(String, Vec<(TagHash, String)>)>,
    /// Indexes into `map_tags_by_package`
    current_package_index: Option<usize>,
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
            map_tags_by_package,
            current_package_index: None,
        }
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
                ui.horizontal_centered(|ui| {
                    ui.vertical(|ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([true, false])
                            .id_salt("map_list_packages")
                            .show(ui, |ui| {
                                for (i, (package_name, _map_tags)) in
                                    self.map_tags_by_package.iter().enumerate()
                                {
                                    if if self.current_package_index == Some(i) {
                                        DButton::new_white(package_name)
                                    } else {
                                        DButton::new(package_name)
                                    }
                                    .min_size(vec2(512.0, 32.0))
                                    .ui(ui)
                                    .clicked()
                                    {
                                        self.current_package_index = Some(i);
                                    }
                                }
                            });
                    });

                    ui.separator();

                    ui.vertical(|ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("map_list_maps")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let current_index = match self.current_package_index {
                                    Some(i) => i,
                                    None => return,
                                };

                                for (tag, name) in self.map_tags_by_package[current_index].1.iter()
                                {
                                    if DButton::new(format!("{name} ({tag})"))
                                        .min_size(vec2(512.0, 32.0))
                                        .ui(ui)
                                        .clicked()
                                    {
                                        match MapTab::new(*tag) {
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
                });
            });

        result
    }
}
