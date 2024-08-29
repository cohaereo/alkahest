use alkahest_renderer::resources::AppResources;
use egui::Context;
use winit::window::Window;

use crate::gui::context::{GuiCtx, GuiView, HiddenWindows, ViewResult};

pub struct PuffinProfiler;

impl GuiView for PuffinProfiler {
    fn draw(
        &mut self,
        ctx: &Context,
        _window: &Window,
        resources: &AppResources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        let mut windows = resources.get_mut::<HiddenWindows>();
        egui::Window::new("Profiler")
            .open(&mut windows.cpu_profiler)
            .show(ctx, |ui| {
                puffin_egui::profiler_ui(ui);
            });

        None
    }
}
