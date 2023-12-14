use std::sync::Arc;

use destiny_pkg::TagHash;
use eframe::{
    egui::{self, RichText},
    epaint::ahash::HashMap,
};
use itertools::Itertools;

use crate::{packages::package_manager, scanner::TagCache, tagtypes::TagType};

use super::{common::tag_context, tag::format_tag_entry, View, ViewAction};

pub struct RawStringsView {
    strings: Vec<(String, Vec<TagHash>)>,
    strings_vec_filtered: Vec<(usize, String, Vec<TagHash>)>,

    string_filter: String,
    selected_stringset: usize,
}

impl RawStringsView {
    pub fn new(cache: Arc<TagCache>) -> Self {
        let mut strings: HashMap<String, Vec<TagHash>> = Default::default();

        for (t, s) in cache
            .hashes
            .iter()
            .flat_map(|(t, sc)| sc.raw_strings.iter().map(|s| (*t, s.clone())))
        {
            match strings.entry(s) {
                std::collections::hash_map::Entry::Occupied(mut o) => o.get_mut().push(t),
                std::collections::hash_map::Entry::Vacant(v) => {
                    v.insert(vec![t]);
                }
            };
        }

        let strings = strings.into_iter().collect_vec();

        Self {
            strings_vec_filtered: strings
                .iter()
                .enumerate()
                .map(|(i, (k, v))| (i, k.clone(), v.clone()))
                .collect(),
            strings,
            string_filter: String::new(),
            selected_stringset: usize::MAX,
        }
    }
}

impl View for RawStringsView {
    fn view(
        &mut self,
        _ctx: &eframe::egui::Context,
        ui: &mut eframe::egui::Ui,
    ) -> Option<super::ViewAction> {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.style_mut().wrap = Some(false);
            ui.horizontal(|ui| {
                ui.label("Search:");
                if ui.text_edit_singleline(&mut self.string_filter).changed() {
                    self.strings_vec_filtered = if !self.string_filter.is_empty() {
                        self.strings
                            .iter()
                            .enumerate()
                            .filter(|(_, (s, _))| {
                                s.to_lowercase()
                                    .contains(&self.string_filter.to_lowercase())
                            })
                            .map(|(i, (k, v))| (i, k.clone(), v.clone()))
                            .collect()
                    } else {
                        self.strings
                            .iter()
                            .enumerate()
                            .map(|(i, (k, v))| (i, k.clone(), v.clone()))
                            .collect_vec()
                    };
                }
            });

            let string_height = {
                let s = ui.spacing();
                s.interact_size.y
            };

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show_rows(
                    ui,
                    string_height,
                    self.strings_vec_filtered.len(),
                    |ui, range| {
                        for (i, string, hashes) in self.strings_vec_filtered[range].iter() {
                            let response = ui
                                .selectable_label(
                                    *i == self.selected_stringset,
                                    format!(
                                        "'{}' {}",
                                        truncate_string_stripped(string, 192),
                                        if hashes.len() > 1 {
                                            format!("({} occurrences)", hashes.len())
                                        } else {
                                            String::new()
                                        }
                                    ),
                                )
                                .on_hover_text(string)
                                .context_menu(|ui| {
                                    if ui.selectable_label(false, "Copy string").clicked() {
                                        ui.output_mut(|o| o.copied_text = string.clone());
                                        ui.close_menu();
                                    }
                                });

                            if response.clicked() {
                                self.selected_stringset = *i;
                            }
                        }
                    },
                );
        });

        if self.selected_stringset < self.strings.len() {
            egui::SidePanel::right("raw_strings_right_panel")
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_width(f32::INFINITY)
                        .show(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            for tag in &self.strings[self.selected_stringset].1 {
                                if let Some(e) = package_manager().get_entry(*tag) {
                                    let label = format_tag_entry(*tag, Some(&e));
                                    let tag_type =
                                        TagType::from_type_subtype(e.file_type, e.file_subtype);
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
        } else {
            None
        }
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
