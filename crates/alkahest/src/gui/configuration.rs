use alkahest_renderer::{
    camera::{Camera, CameraProjection},
    icons::{ICON_CURSOR_POINTER, ICON_EYE},
    renderer::{RenderDebugView, RenderFeatureVisibility, RendererSettings, RendererShared},
    util::text::StringExt,
};
use egui::{Context, RichText, Rounding, Widget};
use strum::IntoEnumIterator;
use transform_gizmo_egui::{EnumSet, GizmoMode};
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
            render_feat_vis_select(ui, "Statics", &mut settings.feature_statics);
            render_feat_vis_select(ui, "Terrain", &mut settings.feature_terrain);
            render_feat_vis_select(ui, "Dynamics", &mut settings.feature_dynamics);
            render_feat_vis_select(ui, "Sky Objects", &mut settings.feature_sky);
            render_feat_vis_select(ui, "Trees/Decorators", &mut settings.feature_decorators);
            render_feat_vis(ui, "Atmosphere", &mut settings.feature_atmosphere);
            render_feat_vis(ui, "Cubemaps", &mut settings.feature_cubemaps);

            ui.separator();
            ui.heading("Render Stages");
            ui.checkbox(&mut settings.stage_transparent, "Transparents");
            ui.checkbox(&mut settings.stage_decals, "Decals");
            ui.checkbox(&mut settings.stage_decals_additive, "Decals (additive)");

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

#[derive(Default, PartialEq)]
pub enum SelectionGizmoMode {
    #[default]
    Select,
    Translate,
    Rotate,
    Scale,
}

impl SelectionGizmoMode {
    pub fn to_enumset(&self) -> EnumSet<GizmoMode> {
        match self {
            SelectionGizmoMode::Select => EnumSet::empty(),
            SelectionGizmoMode::Translate => EnumSet::only(GizmoMode::Translate),
            SelectionGizmoMode::Rotate => EnumSet::only(GizmoMode::Rotate),
            SelectionGizmoMode::Scale => EnumSet::only(GizmoMode::Scale),
        }
    }
}

fn render_feat_vis_select(ui: &mut egui::Ui, name: &str, mode: &mut RenderFeatureVisibility) {
    ui.horizontal(|ui| {
        ui.label(name);

        let rounding_l = Rounding {
            ne: 0.0,
            se: 0.0,
            nw: 2.0,
            sw: 2.0,
        };
        let rounding_r = Rounding {
            nw: 0.0,
            sw: 0.0,
            ne: 2.0,
            se: 2.0,
        };

        ui.style_mut().spacing.item_spacing = [0.0; 2].into();
        ui.style_mut().spacing.button_padding = [4.0, 0.0].into();

        ui.style_mut().visuals.widgets.active.rounding = rounding_l;
        ui.style_mut().visuals.widgets.hovered.rounding = rounding_l;
        ui.style_mut().visuals.widgets.inactive.rounding = rounding_l;

        if ui
            .selectable_label(
                mode.contains(RenderFeatureVisibility::VISIBLE),
                ICON_EYE.to_string(),
            )
            .clicked()
        {
            *mode ^= RenderFeatureVisibility::VISIBLE;
        }

        ui.style_mut().visuals.widgets.active.rounding = rounding_r;
        ui.style_mut().visuals.widgets.hovered.rounding = rounding_r;
        ui.style_mut().visuals.widgets.inactive.rounding = rounding_r;

        ui.add_enabled_ui(mode.contains(RenderFeatureVisibility::VISIBLE), |ui| {
            if ui
                .selectable_label(
                    mode.contains(RenderFeatureVisibility::SELECTABLE),
                    ICON_CURSOR_POINTER.to_string(),
                )
                .clicked()
            {
                *mode ^= RenderFeatureVisibility::SELECTABLE;
            }
        });
    });
}

fn render_feat_vis(ui: &mut egui::Ui, name: &str, visible: &mut bool) {
    ui.horizontal(|ui| {
        ui.label(name);

        let rounding = Rounding {
            nw: 2.0,
            sw: 2.0,
            ne: 2.0,
            se: 2.0,
        };

        ui.style_mut().spacing.button_padding = [4.0, 0.0].into();

        ui.style_mut().visuals.widgets.active.rounding = rounding;
        ui.style_mut().visuals.widgets.hovered.rounding = rounding;
        ui.style_mut().visuals.widgets.inactive.rounding = rounding;

        if ui
            .selectable_label(*visible, ICON_EYE.to_string())
            .clicked()
        {
            *visible = !*visible;
        }
    });
}
