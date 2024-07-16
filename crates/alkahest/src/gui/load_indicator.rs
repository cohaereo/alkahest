use alkahest_renderer::{renderer::RendererShared, resources::Resources};
use egui::{Color32, Context, RichText};
use winit::window::Window;

use crate::{
    gui::{
        bottom_bar::LoadingIcon,
        context::{GuiCtx, GuiView, ViewResult},
    },
    maplist::{MapList, MapLoadState},
};

pub struct ResourceLoadIndicatorOverlay;

impl GuiView for ResourceLoadIndicatorOverlay {
    fn draw(
        &mut self,
        ctx: &Context,
        _window: &Window,
        resources: &Resources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        let renderer = resources.get::<RendererShared>();
        let am = &renderer.data.lock().asset_manager;
        let open = !am.is_idle();

        if open {
            egui::Window::new("Loading")
                .anchor(egui::Align2::RIGHT_TOP, [-12.0, 32.0])
                .title_bar(false)
                .show(ctx, |ui| {
                    self.show_indicator(
                        ui,
                        format!("Loading {} resources", am.remaining_requests()),
                    );
                });
        }

        let maplist = resources.get::<MapList>();
        if let Some(map) = maplist.current_map() {
            if map.load_state == MapLoadState::Loading {
                egui::Window::new("Loading...")
                    .title_bar(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.heading(format!("Loading map '{}'", map.name));
                        })
                    });
            }
        }

        None
    }
}

impl ResourceLoadIndicatorOverlay {
    fn show_indicator<L: AsRef<str>>(&self, ui: &mut egui::Ui, label: L) {
        ui.label(
            RichText::new(format!(
                "{} {}",
                LoadingIcon::Clock.get_frame(),
                label.as_ref(),
            ))
            .size(16.0)
            .color(Color32::WHITE),
        );
    }
}
