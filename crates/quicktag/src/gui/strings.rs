use std::sync::Arc;

use destiny_pkg::TagHash;
use eframe::egui::{self, RichText};
use itertools::Itertools;

use crate::{
    packages::package_manager,
    scanner::TagCache,
    tagtypes::TagType,
    text::{StringCache, StringCacheVec},
};

use super::{common::tag_context, tag::format_tag_entry, View, ViewAction};

pub struct StringsView {
    cache: Arc<TagCache>,
    strings: Arc<StringCache>,
    strings_vec_filtered: StringCacheVec,

    selected_string: u32,
    string_selected_entries: Vec<(TagHash, String, TagType)>,
    string_filter: String,
}

impl StringsView {
    pub fn new(strings: Arc<StringCache>, cache: Arc<TagCache>) -> Self {
        let strings_vec_filtered: StringCacheVec =
            strings.iter().map(|(k, v)| (*k, v.clone())).collect();

        Self {
            cache,
            strings,
            strings_vec_filtered,
            selected_string: u32::MAX,
            string_filter: String::new(),
            string_selected_entries: vec![],
        }
    }
}

impl View for StringsView {
    fn view(
        &mut self,
        _ctx: &eframe::egui::Context,
        ui: &mut eframe::egui::Ui,
    ) -> Option<super::ViewAction> {
        egui::SidePanel::left("strings_left_panel")
            .resizable(true)
            .min_width(384.0)
            .show_inside(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    if ui.text_edit_singleline(&mut self.string_filter).changed() {
                        self.strings_vec_filtered = if !self.string_filter.is_empty() {
                            self.strings
                                .iter()
                                .filter(|(_, s)| {
                                    s.iter()
                                        .any(|s| s.to_lowercase().contains(&self.string_filter))
                                })
                                .map(|(k, v)| (*k, v.clone()))
                                .collect()
                        } else {
                            self.strings
                                .iter()
                                .map(|(k, v)| (*k, v.clone()))
                                .collect_vec()
                        };
                    }
                });

                let string_height = {
                    let s = ui.spacing();
                    s.interact_size.y
                };

                egui::ScrollArea::vertical()
                    .max_width(ui.available_width() * 0.70)
                    .show_rows(
                        ui,
                        string_height,
                        self.strings_vec_filtered.len(),
                        |ui, range| {
                            for (hash, strings) in &self.strings_vec_filtered[range] {
                                let response = if strings.len() > 1 {
                                    ui.selectable_value(
                                        &mut self.selected_string,
                                        *hash,
                                        format!(
                                            "'{}' {:08x} ({} collisions)",
                                            truncate_string_stripped(&strings[0], 192),
                                            hash,
                                            strings.len()
                                        ),
                                    )
                                    .on_hover_text(
                                        strings.iter().map(|s| s.replace('\n', "\\n")).join("\n\n"),
                                    )
                                } else {
                                    ui.selectable_value(
                                        &mut self.selected_string,
                                        *hash,
                                        format!(
                                            "'{}' {:08x}",
                                            truncate_string_stripped(&strings[0], 192),
                                            hash
                                        ),
                                    )
                                    .on_hover_text(strings[0].clone())
                                }
                                .context_menu(|ui| {
                                    if ui.selectable_label(false, "Copy string").clicked() {
                                        ui.output_mut(|o| o.copied_text = strings[0].clone());
                                        ui.close_menu();
                                    }
                                });

                                if response.clicked() {
                                    self.string_selected_entries = vec![];
                                    for (tag, _) in self.cache.hashes.iter().filter(|v| {
                                        v.1.string_hashes.iter().any(|c| c.hash == *hash)
                                    }) {
                                        if let Some(e) = package_manager().get_entry(*tag) {
                                            let label = format_tag_entry(*tag, Some(&e));

                                            self.string_selected_entries.push((
                                                *tag,
                                                label,
                                                TagType::from_type_subtype(
                                                    e.file_type,
                                                    e.file_subtype,
                                                ),
                                            ));
                                        }
                                    }
                                }
                            }
                        },
                    );
            });

        egui::CentralPanel::default()
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_width(f32::INFINITY)
                    .show(ui, |ui| {
                        ui.style_mut().wrap = Some(false);
                        if self.selected_string == u32::MAX {
                            ui.label(RichText::new("No string selected").italics());
                        } else {
                            for (tag, label, tag_type) in &self.string_selected_entries {
                                if ui
                                    .add(egui::SelectableLabel::new(
                                        false,
                                        RichText::new(label).color(tag_type.display_color()),
                                    ))
                                    .context_menu(|ui| tag_context(ui, *tag, None))
                                    .clicked()
                                {
                                    return Some(ViewAction::OpenTag(*tag));
                                }
                            }
                        }
                        None
                    })
                    .inner
            })
            .inner
    }
}

fn truncate_string_stripped(s: &str, max_length: usize) -> String {
    let s = s.replace('\n', "\\n");

    if s.len() >= max_length {
        format!("{}...", s.chars().take(max_length).collect::<String>())
    } else {
        s.to_string()
    }
}
