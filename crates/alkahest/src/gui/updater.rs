use alkahest_renderer::icons::{
    ICON_CANCEL, ICON_DOWNLOAD, ICON_LIGHTNING_BOLT, ICON_SHIELD_HALF_FULL,
};
use anyhow::Context;
use egui::{Align2, Color32, Id, RichText, Rounding, Vec2};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use poll_promise::Promise;

use super::UiExt;
use crate::{
    config,
    gui::bottom_bar::LoadingIcon,
    updater::{self, AvailableUpdate, UpdateChannel, UpdateCheck},
    util::error::{show_error_alert, ErrorAlert},
};

#[derive(Default)]
pub struct ChannelSelector {
    pub open: bool,
}

impl ChannelSelector {
    pub fn show(&mut self, ctx: &egui::Context, resources: &mut crate::resources::Resources) {
        if self.open {
            ctx.layer_painter(egui::LayerId::new(
                egui::Order::Middle,
                Id::new("update_channel_selector_bg").with("layer"),
            ))
            .rect_filled(
                egui::Rect::EVERYTHING,
                Rounding::default(),
                Color32::from_black_alpha(128),
            );

            egui::Area::new(egui::Id::new("Update Channel"))
                .order(egui::Order::Foreground)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    egui::Frame::window(&ctx.style()).show(ui, |ui| {
                        ui.label(
                            "Select the update channel you want to use.\nThis determines the \
                             frequency and stability of updates.",
                        );
                        let mut persist_and_close = false;
                        if ui
                            .big_button_with_icon_color(
                                ICON_SHIELD_HALF_FULL,
                                "Stable",
                                Color32::from_rgb(34, 143, 237),
                            )
                            .on_hover_text(
                                "Stable releases are well-tested and recommended for most users.",
                            )
                            .clicked()
                        {
                            config::with_mut(|c| c.update_channel = Some(UpdateChannel::Stable));
                            persist_and_close = true;
                        }
                        if ui
                            .big_button_with_icon_color(
                                ICON_LIGHTNING_BOLT,
                                "Nightly",
                                Color32::from_rgb(255, 210, 40),
                            )
                            .on_hover_text(
                                "Nightly releases are built from the latest code and may be \
                                 unstable.",
                            )
                            .clicked()
                        {
                            config::with_mut(|c| c.update_channel = Some(UpdateChannel::Nightly));
                            persist_and_close = true;
                        }
                        if ui
                            .big_button_with_icon_color(
                                ICON_CANCEL,
                                "Disable Updates",
                                Color32::from_rgb(213, 86, 86),
                            )
                            .clicked()
                        {
                            config::with_mut(|c| c.update_channel = Some(UpdateChannel::Disabled));
                            persist_and_close = true;
                        }

                        if persist_and_close {
                            config::persist();
                            self.open = false;
                            resources.get_mut::<UpdateCheck>().start(config::with(|c| {
                                c.update_channel.unwrap_or(UpdateChannel::Stable)
                            }));
                        }
                    });
                });
        }
    }
}

pub struct UpdateDownload {
    version: AvailableUpdate,
    markdown_cache: CommonMarkCache,
    download_promise: Option<Promise<anyhow::Result<Vec<u8>>>>,
}

impl UpdateDownload {
    pub fn new(version: AvailableUpdate) -> Self {
        Self {
            markdown_cache: CommonMarkCache::default(),
            version,
            download_promise: None,
        }
    }

    pub fn start(&mut self, url: String) {
        self.download_promise = Some(Promise::spawn_async(async move {
            let response = reqwest::get(&url).await?;
            let bytes = response.bytes().await?;
            Ok(bytes.to_vec())
        }));
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        _resources: &mut crate::resources::Resources,
    ) -> bool {
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            Id::new("update_channel_selector_bg").with("layer"),
        ))
        .rect_filled(
            egui::Rect::EVERYTHING,
            Rounding::default(),
            Color32::from_black_alpha(128),
        );

        let mut close = false;
        if self.download_promise.is_none() {
            egui::Area::new(egui::Id::new("Update Available"))
                .order(egui::Order::Foreground)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    egui::Frame::window(&ctx.style()).show(ui, |ui| {
                        ui.heading(
                            egui::RichText::new(format!(
                                "Update available - {}",
                                self.version.version
                            ))
                            .color(Color32::WHITE),
                        );
                        ui.hyperlink_to("View on GitHub", &self.version.url);

                        egui::Frame::group(&ctx.style()).show(ui, |ui| {
                            egui::ScrollArea::new([false, true])
                                .max_width(ui.available_width())
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    CommonMarkViewer::new("changelog_md").show(
                                        ui,
                                        &mut self.markdown_cache,
                                        &self.version.changelog,
                                    );
                                });
                        });

                        ui.horizontal(|ui| {
                            if ui.button(format!("{} Update", ICON_DOWNLOAD)).clicked() {
                                self.start(self.version.download_url.clone());
                            }

                            if ui.button("Later").clicked() {
                                close = true;
                            }
                        });
                    });
                });
        } else {
            egui::Area::new(egui::Id::new("Updating"))
                .order(egui::Order::Foreground)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} Updating Alkahest",
                            LoadingIcon::Clock.get_frame()
                        ))
                        .size(36.0)
                        .color(Color32::WHITE),
                    );
                });

            if self
                .download_promise
                .as_ref()
                .map_or(false, |v| v.poll().is_ready())
            {
                let result = self.download_promise.take().unwrap().block_and_take();
                match result {
                    Ok(bytes) => {
                        let _ = updater::execute_update(bytes)
                            .context("Failed to install update")
                            .err_alert();

                        close = true;
                    }
                    Err(e) => {
                        tokio::spawn(async move {
                            show_error_alert(e.context("Failed to download update"));
                        });
                        close = true;
                    }
                }
            }
        }

        !close
    }
}
