use alkahest_renderer::{
    camera::{Camera, CameraProjection},
    ecs::tags::{NodeFilter, NodeFilterSet},
    icons::{ICON_CURSOR_DEFAULT, ICON_CURSOR_POINTER, ICON_EYE},
    renderer::{RenderDebugView, RenderFeatureVisibility, RendererSettings, RendererShared},
    util::text::StringExt,
};
use egui::{Context, RichText, Rounding, Widget};
use strum::IntoEnumIterator;
use transform_gizmo_egui::{EnumSet, GizmoMode};
use winit::window::Window;

use crate::{
    config, gui::context::{GuiCtx, GuiView, ViewResult}, resources::Resources
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
            config::with_mut(|c|{
            ui.heading("Graphics");
            ui.checkbox(&mut c.renderer.vsync, "VSync");
            ui.checkbox(&mut c.renderer.matcap, "Matcap");
            ui.checkbox(&mut c.renderer.shadows, "Shadows");
            ui.checkbox(&mut c.renderer.ssao, "SSAO");
            ui.collapsing("SSAO Settings", |ui| {
                let renderer = resources.get::<RendererShared>();
                let ssao_data = renderer.ssao.scope.data();
                ui.horizontal(|ui| {
                    ui.label("Radius");
                    egui::DragValue::new(&mut ssao_data.radius)
                        .speed(0.01)
                        .clamp_range(0.0..=10.0)
                        .suffix("m")
                        .ui(ui);
                });

                ui.horizontal(|ui| {
                    ui.label("Bias");
                    egui::DragValue::new(&mut ssao_data.bias)
                        .speed(0.01)
                        .clamp_range(0.0..=10.0)
                        .suffix("m")
                        .ui(ui);
                });
            });
            render_feat_vis(ui, "Node Visualization", &mut c.visual.node_nametags);
            ui.collapsing("Node filters", |ui| {
                let mut filters = resources.get_mut::<NodeFilterSet>();
                for filter in NodeFilter::iter() {
                    let filter_text = RichText::new(format!(
                        "{} {}",
                        filter.icon(),
                        filter.to_string().split_pascalcase()
                    ))
                    .color(filter.color());

                    let mut checked = filters.contains(&filter);
                    if ui.checkbox(&mut checked, filter_text).changed() {
                        if checked {
                            filters.insert(filter);
                            c.visual.node_filters.insert(filter.to_string());
                        } else {
                            filters.remove(&filter);
                            c.visual.node_filters.remove(&filter.to_string());
                        }
                    }
                }
            });

            egui::ComboBox::from_label("Debug View")
                .selected_text(c.renderer.debug_view.to_string().split_pascalcase())
                .show_ui(ui, |ui| {
                    for view in RenderDebugView::iter() {
                        ui.selectable_value(
                            &mut c.renderer.debug_view,
                            view,
                            view.to_string().split_pascalcase(),
                        );
                    }
                });

            ui.separator();
            ui.heading("Feature Renderers");
            render_feat_vis_select(ui, "Statics", &mut c.renderer.feature_statics);
            render_feat_vis_select(ui, "Terrain", &mut c.renderer.feature_terrain);
            render_feat_vis_select(ui, "Dynamics", &mut c.renderer.feature_dynamics);
            render_feat_vis_select(ui, "Sky Objects", &mut c.renderer.feature_sky);
            render_feat_vis_select(ui, "Water", &mut c.renderer.feature_water);
            render_feat_vis_select(ui, "Trees/Decorators", &mut c.renderer.feature_decorators);
            render_feat_vis(ui, "Atmosphere", &mut c.renderer.feature_atmosphere);
            render_feat_vis(ui, "Cubemaps", &mut c.renderer.feature_cubemaps);
            render_feat_vis(ui, "Global Lighting", &mut c.renderer.feature_global_lighting);

            ui.separator();
            ui.heading("Render Stages");
            ui.checkbox(&mut c.renderer.stage_transparent, "Transparents");
            ui.checkbox(&mut c.renderer.stage_decals, "Decals");
            ui.checkbox(&mut c.renderer.stage_decals_additive, "Decals (additive)");

            resources
                .get::<RendererShared>()
                .set_render_settings(c.renderer.clone());

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
        })});

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
                    ICON_CURSOR_DEFAULT.to_string(),
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
