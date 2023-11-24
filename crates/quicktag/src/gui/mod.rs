mod common;
mod tag;

use std::sync::Arc;

use destiny_pkg::{package::UEntryHeader, PackageNamedTagEntry, PackageVersion, TagHash};
use eframe::{
    egui::{self},
    emath::Align2,
    epaint::{Color32, Rounding, Vec2},
};
use egui_notify::Toasts;
use poll_promise::Promise;

use crate::{
    packages::package_manager,
    scanner::{load_tag_cache, scanner_progress, ScanStatus, TagCache},
    tagtypes::TagType,
    text::{create_stringmap, StringCache},
};

use self::{
    common::tag_context,
    tag::{format_tag_entry, TagView},
};

#[derive(PartialEq)]
pub enum Panel {
    Tag,
    NamedTags,
}

pub struct QuickTagApp {
    cache_load: Option<Promise<TagCache>>,
    cache: Arc<TagCache>,
    strings: Arc<StringCache>,

    tag_input: String,

    toasts: Toasts,

    open_panel: Panel,

    tag_view: Option<TagView>,
    named_tags: NamedTags,
    named_tag_filter: String,
}

impl QuickTagApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>, version: PackageVersion) -> Self {
        QuickTagApp {
            cache_load: Some(Promise::spawn_thread("load_cache", move || {
                load_tag_cache(version)
            })),
            cache: Default::default(),
            strings: Arc::new(create_stringmap().unwrap()),
            tag_view: None,
            tag_input: String::new(),
            toasts: Toasts::default(),

            open_panel: Panel::Tag,
            named_tags: NamedTags::new(),
            named_tag_filter: String::new(),
        }
    }
}

impl eframe::App for QuickTagApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(cache_promise) = self.cache_load.as_ref() {
            if cache_promise.poll().is_pending() {
                {
                    let painter = ctx.layer_painter(egui::LayerId::background());
                    painter.rect_filled(
                        egui::Rect::EVERYTHING,
                        Rounding::default(),
                        Color32::from_black_alpha(127),
                    );
                }
                egui::Window::new("Loading cache")
                    .collapsible(false)
                    .resizable(false)
                    .title_bar(false)
                    .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                    .show(ctx, |ui| {
                        let progress = if let ScanStatus::Scanning {
                            current_package,
                            total_packages,
                        } = scanner_progress()
                        {
                            current_package as f32 / total_packages as f32
                        } else {
                            0.9999
                        };

                        ui.add(
                            egui::ProgressBar::new(progress)
                                .animate(true)
                                .text(scanner_progress().to_string()),
                        );
                    });
            }
        }

        if self
            .cache_load
            .as_ref()
            .map(|v| v.poll().is_ready())
            .unwrap_or_default()
        {
            let c = self.cache_load.take().unwrap();
            let cache = c.try_take().unwrap_or_default();
            self.cache = Arc::new(cache);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Tag:");
                let submitted = ui.text_edit_singleline(&mut self.tag_input).lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Open").clicked() || submitted {
                    let tag = if self.tag_input.len() >= 16 {
                        let hash = u64::from_str_radix(&self.tag_input, 16).unwrap_or_default();
                        if let Some(t) = package_manager().hash64_table.get(&u64::from_be(hash)) {
                            t.hash32
                        } else {
                            TagHash::NONE
                        }
                    } else {
                        let hash = u32::from_str_radix(&self.tag_input, 16).unwrap_or_default();
                        TagHash(u32::from_be(hash))
                    };

                    self.open_tag(tag);
                }
            });

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.open_panel, Panel::Tag, "Tag");
                ui.selectable_value(&mut self.open_panel, Panel::NamedTags, "Named tags");
            });

            ui.separator();

            match self.open_panel {
                Panel::Tag => {
                    if let Some(tagview) = &mut self.tag_view {
                        tagview.view(ctx, ui);
                    } else {
                        ui.label("No tag loaded");
                    }
                }
                Panel::NamedTags => {
                    self.named_tags_panel(ui);
                }
            }
        });

        self.toasts.show(ctx);
    }
}

impl QuickTagApp {
    fn open_tag(&mut self, tag: TagHash) {
        let new_view = TagView::create(self.cache.clone(), self.strings.clone(), tag);
        if new_view.is_some() {
            self.tag_view = new_view;
            self.open_panel = Panel::Tag;
        } else if package_manager().get_entry(tag).is_some() {
            self.toasts.warning(format!(
                "Could not find tag '{}' ({tag}) in cache\nThis usually means it has no references",
                self.tag_input
            ));
        } else {
            self.toasts
                .error(format!("Could not find tag '{}' ({tag})", self.tag_input));
        }
    }

    fn named_tags_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.named_tag_filter);
        });

        egui::ScrollArea::vertical()
            .max_width(f32::INFINITY)
            .show(ui, |ui| {
                for i in 0..self.named_tags.tags.len() {
                    let (entry, nt) = self.named_tags.tags[i].clone();
                    if !nt.name.to_lowercase().contains(&self.named_tag_filter) {
                        continue;
                    }

                    let tagtype = TagType::from_type_subtype(entry.file_type, entry.file_subtype);

                    let fancy_tag = format_tag_entry(nt.hash, Some(&entry));

                    let tag_label = egui::RichText::new(format!("{} {fancy_tag}", nt.name))
                        .color(tagtype.display_color());

                    if ui
                        .add(egui::SelectableLabel::new(false, tag_label))
                        .context_menu(|ui| tag_context(ui, nt.hash, None))
                        .clicked()
                    {
                        self.open_tag(nt.hash);
                    }
                }
            });
    }
}

pub trait View {
    fn view(&mut self, ctx: &egui::Context, ui: &mut egui::Ui);
}

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
