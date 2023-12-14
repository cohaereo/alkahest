use std::path::PathBuf;

use destiny_pkg::TagHash;
use eframe::egui::{self, RichText};

use crate::{packages::package_manager, tagtypes::TagType};

use super::{common::tag_context, tag::format_tag_entry, View, ViewAction};

pub struct PackagesView {
    selected_package: u16,
    package_entry_search_cache: Vec<(String, TagType)>,
    package_filter: String,
    package_entry_filter: String,
}

impl PackagesView {
    pub fn new() -> Self {
        Self {
            selected_package: u16::MAX,
            package_entry_search_cache: vec![],
            package_filter: String::new(),
            package_entry_filter: String::new(),
        }
    }
}

impl View for PackagesView {
    fn view(
        &mut self,
        _ctx: &eframe::egui::Context,
        ui: &mut eframe::egui::Ui,
    ) -> Option<super::ViewAction> {
        egui::SidePanel::left("packages_left_panel")
            .resizable(true)
            .min_width(256.0)
            .show_inside(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.package_filter);
                });
                egui::ScrollArea::vertical()
                    .max_width(f32::INFINITY)
                    .show(ui, |ui| {
                        for (id, path) in package_manager().package_paths.iter() {
                            let path_stem = PathBuf::from(path)
                                .file_stem()
                                .unwrap()
                                .to_string_lossy()
                                .to_string();

                            if !self.package_filter.is_empty()
                                && !path_stem
                                    .to_lowercase()
                                    .contains(&self.package_filter.to_lowercase())
                            {
                                continue;
                            }

                            if ui
                                .selectable_value(
                                    &mut self.selected_package,
                                    *id,
                                    format!("{id:04x}: {path_stem}"),
                                )
                                .changed()
                            {
                                self.package_entry_search_cache = vec![];
                                if let Ok(p) = package_manager().version.open(path) {
                                    for (i, e) in p.entries().iter().enumerate() {
                                        let label =
                                            format_tag_entry(TagHash::new(*id, i as u16), Some(e));

                                        self.package_entry_search_cache.push((
                                            label,
                                            TagType::from_type_subtype(e.file_type, e.file_subtype),
                                        ));
                                    }
                                }
                            }
                        }
                    });
            });

        egui::CentralPanel::default()
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.package_entry_filter);
                });
                egui::ScrollArea::vertical()
                    .max_width(f32::INFINITY)
                    .show(ui, |ui| {
                        ui.style_mut().wrap = Some(false);

                        if self.selected_package == u16::MAX {
                            ui.label(RichText::new("No package selected").italics());
                        } else {
                            for (i, (label, tag_type)) in self
                                .package_entry_search_cache
                                .iter()
                                .enumerate()
                                .filter(|(_, (label, _))| {
                                    self.package_entry_filter.is_empty()
                                        || label.to_lowercase().contains(&self.package_entry_filter)
                                })
                            {
                                let tag = TagHash::new(self.selected_package, i as u16);
                                if ui
                                    .add(egui::SelectableLabel::new(
                                        false,
                                        RichText::new(format!("{i}: {label}"))
                                            .color(tag_type.display_color()),
                                    ))
                                    .context_menu(|ui| tag_context(ui, tag, None))
                                    .clicked()
                                {
                                    return Some(ViewAction::OpenTag(tag));
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
