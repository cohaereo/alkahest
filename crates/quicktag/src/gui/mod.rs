mod common;
mod dxgi;
mod tag;
mod texture;

use std::path::PathBuf;
use std::sync::Arc;

use destiny_pkg::{package::UEntryHeader, PackageNamedTagEntry, PackageVersion, TagHash};
use eframe::egui::RichText;
use eframe::egui_wgpu::RenderState;
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
    Packages,
}

pub struct QuickTagApp {
    cache_load: Option<Promise<TagCache>>,
    cache: Arc<TagCache>,
    strings: Arc<StringCache>,

    tag_input: String,

    toasts: Toasts,

    open_panel: Panel,

    tag_view: Option<TagView>,

    // TODO(cohae): Split named tag panel to it's own view
    named_tags: NamedTags,
    named_tag_filter: String,

    // TODO(cohae): Split package panel to it's own view
    selected_package: u16,
    package_entry_search_cache: Vec<(String, TagType)>,
    package_filter: String,
    package_entry_filter: String,

    pub wgpu_state: RenderState,
}

impl QuickTagApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>, version: PackageVersion) -> Self {
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

            selected_package: u16::MAX,
            package_entry_search_cache: vec![],
            package_filter: String::new(),
            package_entry_filter: String::new(),

            wgpu_state: cc.wgpu_render_state.clone().unwrap(),
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
                    let tag_input_trimmed = self.tag_input.trim();
                    let tag = if tag_input_trimmed.len() >= 16 {
                        let hash = u64::from_str_radix(tag_input_trimmed, 16).unwrap_or_default();
                        if let Some(t) = package_manager().hash64_table.get(&u64::from_be(hash)) {
                            t.hash32
                        } else {
                            TagHash::NONE
                        }
                    } else {
                        let hash = u32::from_str_radix(tag_input_trimmed, 16).unwrap_or_default();
                        TagHash(u32::from_be(hash))
                    };

                    self.open_tag(tag);
                }
            });

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.open_panel, Panel::Tag, "Tag");
                ui.selectable_value(&mut self.open_panel, Panel::NamedTags, "Named tags");
                ui.selectable_value(&mut self.open_panel, Panel::Packages, "Packages");
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
                Panel::Packages => {
                    self.packages_panel(ui);
                }
            }
        });

        self.toasts.show(ctx);
    }
}

impl QuickTagApp {
    fn open_tag(&mut self, tag: TagHash) {
        let new_view = TagView::create(
            self.cache.clone(),
            self.strings.clone(),
            tag,
            self.wgpu_state.clone(),
        );
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

                    let tag_label = egui::RichText::new(fancy_tag).color(tagtype.display_color());

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

    fn packages_panel(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::left("packages_left_panel")
            .resizable(true)
            .min_width(256.0)
            .show_inside(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                egui::ScrollArea::vertical()
                    .max_width(f32::INFINITY)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Search:");
                            ui.text_edit_singleline(&mut self.package_filter);
                        });
                        for (id, path) in package_manager().package_paths.iter() {
                            let path_stem = PathBuf::from(path)
                                .file_stem()
                                .unwrap()
                                .to_string_lossy()
                                .to_string();

                            if !self.package_filter.is_empty()
                                && !path_stem.to_lowercase().contains(&self.package_filter)
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
                                } else {
                                    self.toasts.error(format!("Failed to open package {path}"));
                                }
                            }
                        }
                    });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_width(f32::INFINITY)
                .show(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        ui.text_edit_singleline(&mut self.package_entry_filter);
                    });

                    if self.selected_package == u16::MAX {
                        ui.label(RichText::new("No package selected").italics());
                    } else {
                        let mut open_tag = None;
                        for (i, (label, tag_type)) in
                            self.package_entry_search_cache.iter().enumerate().filter(
                                |(_, (label, _))| {
                                    self.package_entry_filter.is_empty()
                                        || label.to_lowercase().contains(&self.package_entry_filter)
                                },
                            )
                        {
                            let tag = TagHash::new(self.selected_package, i as u16);
                            if ui
                                .add(egui::SelectableLabel::new(
                                    false,
                                    RichText::new(label).color(tag_type.display_color()),
                                ))
                                .context_menu(|ui| tag_context(ui, tag, None))
                                .clicked()
                            {
                                open_tag = Some(tag);
                            }
                        }

                        if let Some(tag) = open_tag {
                            self.open_tag(tag);
                        }
                    }
                });
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
