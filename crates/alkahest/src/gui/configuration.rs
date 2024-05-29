use alkahest_renderer::{
    camera::{Camera, CameraProjection},
    renderer::{RenderDebugView, RendererSettings, RendererShared},
    util::text::StringExt,
};
use egui::{Context, RichText, Widget};
use strum::IntoEnumIterator;
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
        egui::Window::new("Settings").show(ctx, |ui| {
            ui.heading("Graphics");
            let mut settings = resources.get_mut::<RendererSettings>();
            ui.checkbox(&mut settings.vsync, "VSync");
            ui.checkbox(&mut settings.ssao, "SSAO");
            ui.checkbox(&mut settings.matcap, "Matcap");
            ui.checkbox(&mut settings.shadows, "Shadows");

            egui::ComboBox::from_label("Debug View")
                .selected_text(settings.debug_view.to_string().split_pascalcase())
                .show_ui(ui, |ui| {
                    for view in RenderDebugView::iter() {
                        ui.selectable_value(
                            &mut settings.debug_view,
                            view,
                            view.to_string().split_pascalcase(),
                        );
                    }
                });

            ui.separator();
            ui.heading("Feature Renderers");
            ui.checkbox(&mut settings.feature_statics, "Statics");
            ui.checkbox(&mut settings.feature_terrain, "Terrain");
            ui.checkbox(&mut settings.feature_dynamics, "Dynamics");
            ui.checkbox(&mut settings.feature_sky, "Sky Objects");
            ui.checkbox(&mut settings.feature_decorators, "Trees/Decorators");
            ui.checkbox(&mut settings.feature_atmosphere, "Atmosphere");
            ui.checkbox(&mut settings.feature_cubemaps, "Cubemaps");

            resources
                .get::<RendererShared>()
                .set_render_settings(settings.clone());

            ui.separator();

            let mut camera = resources.get_mut::<Camera>();
            ui.heading("Camera");
            ui.strong(RichText::new("TODO: move to dropdown button").color(egui::Color32::YELLOW));
            ui.horizontal(|ui| {
                egui::DragValue::new(&mut camera.speed_mul)
                    .clamp_range(0f32..=25.0)
                    .speed(0.05)
                    .ui(ui);
                ui.label("Speed");
            });

            if let CameraProjection::Perspective { fov, .. } = &mut camera.projection {
                ui.horizontal(|ui| {
                    egui::DragValue::new(fov)
                        .clamp_range(5f32..=120.0)
                        .speed(0.05)
                        .ui(ui);
                    ui.label("FOV");
                });
            }

            ui.horizontal(|ui| {
                egui::DragValue::new(&mut camera.smooth_movement)
                    .clamp_range(0f32..=5.0)
                    .speed(0.05)
                    .ui(ui);
                ui.label("Smooth movement");
            });

            ui.horizontal(|ui| {
                egui::DragValue::new(&mut camera.smooth_look)
                    .clamp_range(0f32..=5.0)
                    .speed(0.05)
                    .ui(ui);
                ui.label("Smooth look");
            });
        });

        None
    }
}
