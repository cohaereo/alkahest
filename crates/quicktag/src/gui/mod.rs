mod common;
mod tag;

use std::sync::Arc;

use destiny_pkg::TagHash;
use eframe::{
    egui::{self},
    emath::Align2,
    epaint::{Color32, Rounding, Vec2},
};
use poll_promise::Promise;

use crate::{
    scanner::{load_tag_cache, scanner_progress, ScanStatus, TagCache},
    text::{create_stringmap, StringCache},
};

use self::tag::TagView;

pub struct QuickTagApp {
    cache_load: Option<Promise<TagCache>>,
    cache: Arc<TagCache>,
    strings: Arc<StringCache>,

    tag_view: Option<TagView>,

    tag_input: String,
}

impl QuickTagApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        QuickTagApp {
            cache_load: Some(Promise::spawn_thread("load_cache", load_tag_cache)),
            cache: Default::default(),
            strings: Arc::new(create_stringmap().unwrap()),
            tag_view: None,
            tag_input: String::new(),
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
                ui.text_edit_singleline(&mut self.tag_input);
                if ui.button("Open").clicked() {
                    let hash = u32::from_str_radix(&self.tag_input, 16).unwrap_or_default();
                    let new_view = TagView::create(
                        self.cache.clone(),
                        self.strings.clone(),
                        TagHash(u32::from_be(hash)),
                    );
                    if new_view.is_some() {
                        self.tag_view = new_view;
                    }
                }
            });

            if let Some(tagview) = &mut self.tag_view {
                tagview.view(ctx, ui);
            }
        });
    }
}

pub trait View {
    fn view(&mut self, ctx: &egui::Context, ui: &mut egui::Ui);
}
