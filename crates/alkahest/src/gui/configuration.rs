use alkahest_renderer::{
    hocus,
    renderer::{RendererSettings, RendererShared},
};
use egui::Context;
use winit::window::Window;

use crate::{
    gui::context::{GuiCtx, GuiView, ViewResult},
    resources::Resources,
};

pub struct RenderSettingsPanel;

impl GuiView for RenderSettingsPanel {
    fn draw(
        &mut self,
        ctx: &Context,
        _window: &Window,
        resources: &Resources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        egui::Window::new("Render Settings").show(ctx, |ui| {
            let mut settings = resources.get_mut::<RendererSettings>();
            ui.checkbox(&mut settings.vsync, "VSync");
            ui.checkbox(&mut settings.ssao, "SSAO");
            ui.checkbox(&mut settings.atmosphere, "Atmosphere");
            ui.checkbox(&mut settings.matcap, "Matcap");

            let renderer = resources.get::<RendererShared>();
            renderer.set_render_settings(settings.clone());
        });

        None
    }
}
