use destiny_pkg::{package::UEntryHeader, PackageNamedTagEntry};
use eframe::egui::{self, RichText};

use crate::{packages::package_manager, tagtypes::TagType};

use super::{common::tag_context, tag::format_tag_entry, View, ViewAction};

pub struct NamedTags {
    pub tags: Vec<(UEntryHeader, PackageNamedTagEntry)>,
}

impl NamedTags {
    pub fn new() -> NamedTags {
        NamedTags {
            tags: package_manager()
                .named_tags
                .iter()
                .cloned()
                .filter_map(|n| Some((package_manager().get_entry(n.hash)?, n)))
                .collect(),
        }
    }
}
pub struct NamedTagView {
    named_tags: NamedTags,
    named_tag_filter: String,
}

impl NamedTagView {
    pub fn new() -> Self {
        Self {
            named_tags: NamedTags::new(),
            named_tag_filter: String::new(),
        }
    }
}

impl View for NamedTagView {
    fn view(
        &mut self,
        _ctx: &eframe::egui::Context,
        ui: &mut eframe::egui::Ui,
    ) -> Option<super::ViewAction> {
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.named_tag_filter);
        });

        egui::ScrollArea::vertical()
            .max_width(f32::INFINITY)
            .show(ui, |ui| {
                if self.named_tags.tags.is_empty() {
                    ui.label(RichText::new("No named tags found").italics());
                } else {
                    for i in 0..self.named_tags.tags.len() {
                        let (entry, nt) = self.named_tags.tags[i].clone();
                        if !nt
                            .name
                            .to_lowercase()
                            .contains(&self.named_tag_filter.to_lowercase())
                        {
                            continue;
                        }

                        let tagtype =
                            TagType::from_type_subtype(entry.file_type, entry.file_subtype);

                        let fancy_tag = format_tag_entry(nt.hash, Some(&entry));

                        let tag_label =
                            egui::RichText::new(fancy_tag).color(tagtype.display_color());

                        if ui
                            .add(egui::SelectableLabel::new(false, tag_label))
                            .context_menu(|ui| tag_context(ui, nt.hash, None))
                            .clicked()
                        {
                            return Some(ViewAction::OpenTag(nt.hash));
                        }
                    }
                }

                None
            })
            .inner
    }
}
