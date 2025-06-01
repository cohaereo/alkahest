use std::time::Instant;

use alkahest_renderer::{
    icons::{ICON_CAMERA_OFF_OUTLINE, ICON_EYE_LOCK_OUTLINE},
    resources::AppResources,
    util::FloatExt,
};
use egui::*;
use windows_registry::CURRENT_USER;
use winit::window::Window;

use super::context::{GuiCtx, GuiView, ViewAction};

pub struct Sodi {
    accepted: bool,
    start_time: Instant,
    /// Used to reset timer when drawing starts
    first_draw: bool,
}

impl Sodi {
    const REG_ALKAHEST: &str = "Software\\Alkahest\\";

    /// Disables the popup permanently
    fn disable_popup(&self) {
        if let Ok(key) = CURRENT_USER.create(Self::REG_ALKAHEST) {
            key.set_u32("SodiAccepted", 1).ok();
        }
    }

    fn already_accepted(&self) -> bool {
        match CURRENT_USER.create(Self::REG_ALKAHEST) {
            Ok(key) => key.get_u32("SodiAccepted").unwrap_or_default() != 0,
            Err(e) => {
                error!("Failed to check SODI acceptance: {e:?}");
                false
            }
        }
    }
}

impl Default for Sodi {
    fn default() -> Self {
        Self {
            accepted: false,
            start_time: Instant::now(),
            first_draw: true,
        }
    }
}

impl GuiView for Sodi {
    fn draw(
        &mut self,
        ctx: &Context,
        _window: &Window,
        _resources: &AppResources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewAction> {
        if self.first_draw {
            if self.already_accepted() {
                return Some(ViewAction::Close);
            }

            self.start_time = Instant::now();
            self.first_draw = false;
        }
        // 0s - 1s fade in black background
        // 0.5s - 1.5s fade in frame
        // 1s - 16s timer
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let button_time = elapsed.remap_clamped(1., 16., 15., 0.);

        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            Id::new("update_channel_selector_bg").with("layer"),
        ))
        .rect_filled(
            egui::Rect::EVERYTHING,
            CornerRadius::default(),
            Color32::from_black_alpha(elapsed.remap_clamped(0.25, 1.0, 0.0, 196.0) as u8),
        );

        let mut close = false;
        egui::Area::new(egui::Id::new("Sodi"))
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width(800.0)
            .show(ctx, |ui| {
                egui::Frame::window(&ctx.style())
                    .inner_margin(32.0)
                    .multiply_with_opacity(elapsed.remap_clamped(0.25, 1.0, 0.0, 1.0))
                    .show(ui, |ui| {
                        ui.style_mut().spacing.item_spacing = vec2(42.0, 36.0);
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.style_mut().spacing.item_spacing.y = 12.0;
                                ui.label(
                                    RichText::new(ICON_EYE_LOCK_OUTLINE)
                                        .color(Color32::WHITE)
                                        .size(128.0),
                                );

                                ui.weak(format!("{ICON_CAMERA_OFF_OUTLINE} Capturing disabled"));
                            });

                            ui.vertical(|ui| {
                                ui.style_mut().spacing.item_spacing.y = 12.0;
                                ui.heading(
                                    egui::RichText::new("For your eyes only")
                                        .color(Color32::WHITE)
                                        .strong()
                                        .size(36.0),
                                );
                                ui.add_space(12.0);

                                #[rustfmt::skip]
                                ui.label(
"Alkahest is a tool for artists to use as reference, and while it may seem tempting to use it for exploring unreleased content, you should think twice before doing so, and thrice before sharing leaks with others.

I developed this tool to better understand Destiny’s renderer and VFX system, and that won't change.

I do not condone the use of Alkahest for leaks, spoilers, or any actions that violate Bungie’s Terms of Service.

Using Alkahest to leak content will reduce the likelihood of future public releases, and I may stop its development altogether at any time if misuse continues.

Don't ruin the secrets of the game for yourself or others.
",
                                );
                                ui.strong("By using Alkahest, you agree to keep unreleased content to your eyes only.");

                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut self.accepted, "I agree to not use Alkahest for leaking content");

                                    if button_time > 0. {
                                        ui.add_enabled_ui(false, |ui| {
                                            ui.button(format!("Wait {button_time:.0}s..."))
                                        });
                                    } else {
                                        ui.add_enabled_ui(self.accepted, |ui| {
                                            if ui.button("Accept").clicked() {
                                                self.disable_popup();
                                                close = true;
                                            }
                                        });
                                    }
                                });
                            });
                        });
                    });
            });

        close.then_some(ViewAction::Close)
    }
}
