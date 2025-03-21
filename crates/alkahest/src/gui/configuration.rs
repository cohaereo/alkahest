use alkahest_renderer::{
    camera::{Camera, CameraProjection},
    ecs::tags::{NodeFilter, NodeFilterSet},
    icons::{ICON_CLIPBOARD, ICON_CURSOR_DEFAULT, ICON_EYE},
    renderer::{RenderDebugView, RenderFeatureVisibility, RendererShared, ShadowQuality},
    util::text::StringExt,
};
use egui::{Context, CornerRadius, RichText, Widget};
use strum::IntoEnumIterator;
use transform_gizmo_egui::{EnumSet, GizmoMode};
use winit::window::Window;

use super::console;
use crate::{
    config,
    gui::context::{GuiCtx, GuiView, ViewAction},
    resources::AppResources,
};

pub struct RenderSettingsPanel;

impl GuiView for RenderSettingsPanel {
    fn draw(
        &mut self,
        ctx: &Context,
        _window: &Window,
        resources: &AppResources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewAction> {
        egui::Window::new("Settings").show(ctx, |ui| {
            let mut camera = resources.get_mut::<Camera>();
            ui.heading("Camera");
            ui.strong(RichText::new("TODO: move to dropdown button").color(egui::Color32::YELLOW));
            let position = camera.position();
            let orientation = camera.orientation();
            ui.label(format!(
                "XYZ: {:.2} / {:.3} / {:.2}",
                position.x, position.y, position.z
            ));

            if ui
                .button(format!(
                    "{} Copy goto command{}",
                    ICON_CLIPBOARD,
                    ui.input(|i| i.modifiers.shift)
                        .then_some(" (+angles)")
                        .unwrap_or_default()
                ))
                .clicked()
            {
                let command = if ui.input(|i| i.modifiers.shift) {
                    format!(
                        "goto {} {} {} {} {}",
                        position.x, position.y, position.z, orientation.x, orientation.y,
                    )
                } else {
                    format!("goto {} {} {}", position.x, position.y, position.z)
                };

                ui.ctx().copy_text(command);
            }

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                egui::DragValue::new(&mut camera.speed_mul)
                    .range(0.05f32..=25.0)
                    .speed(0.05)
                    .ui(ui);
                ui.label("Speed");
            });

            if let CameraProjection::Perspective { fov, .. } = &mut camera.projection {
                ui.horizontal(|ui| {
                    egui::DragValue::new(fov)
                        .range(5f32..=120.0)
                        .speed(0.05)
                        .ui(ui);
                    ui.label("FOV");
                });
            }

            ui.horizontal(|ui| {
                egui::DragValue::new(&mut camera.smooth_movement)
                    .range(0f32..=5.0)
                    .speed(0.05)
                    .ui(ui);
                ui.label("Smooth movement");
            });

            ui.horizontal(|ui| {
                egui::DragValue::new(&mut camera.smooth_look)
                    .range(0f32..=5.0)
                    .speed(0.05)
                    .ui(ui);
                ui.label("Smooth look");
            });

            ui.separator();

            config::with_mut(|c| {
                ui.collapsing(RichText::new("Graphics").heading(), |ui| {
                    ui.checkbox(&mut c.renderer.vsync, "VSync");
                    ui.checkbox(&mut c.renderer.matcap, "Matcap");
                    ui.checkbox(&mut c.renderer.draw_selection_outline, "Selection Outline");

                    if egui::ComboBox::from_label("Shadows")
                        .selected_text(c.renderer.shadow_quality.to_string().split_pascalcase())
                        .show_ui(ui, |ui| {
                            let mut changed = false;
                            for quality in ShadowQuality::iter() {
                                changed |= ui
                                    .selectable_value(
                                        &mut c.renderer.shadow_quality,
                                        quality,
                                        quality.to_string().split_pascalcase(),
                                    )
                                    .clicked();
                            }
                            changed
                        })
                        .inner
                        .unwrap_or_default()
                    {
                        console::queue_command("recreate_shadowmaps", &[]);
                    }
                    ui.checkbox(&mut c.renderer.ssao, "SSAO");
                    ui.collapsing("SSAO Settings", |ui| {
                        let renderer = resources.get::<RendererShared>();
                        let ssao_data = renderer.ssao.scope.data();
                        ui.horizontal(|ui| {
                            ui.label("Radius");
                            egui::DragValue::new(&mut ssao_data.radius)
                                .speed(0.01)
                                .range(0.0..=10.0)
                                .suffix("m")
                                .ui(ui);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Bias");
                            egui::DragValue::new(&mut ssao_data.bias)
                                .speed(0.01)
                                .range(0.0..=10.0)
                                .suffix("m")
                                .ui(ui);
                        });
                    });
                    // ui.checkbox(&mut c.renderer.depth_prepass, "⚠ Depth Prepass");

                    render_feat_vis(ui, "Crosshair", &mut c.visual.draw_crosshair);
                    render_feat_vis(ui, "Node Visualization", &mut c.visual.node_nametags);
                    ui.collapsing("Node filters", |ui| {
                        ui.checkbox(
                            &mut c.visual.node_nametags_named_only,
                            "Only show named nodes",
                        );
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
                });

                ui.separator();
                ui.collapsing(RichText::new("Feature Renderers").heading(), |ui| {
                    render_feat_vis_select(ui, "Statics", &mut c.renderer.feature_statics);
                    render_feat_vis_select(ui, "Terrain", &mut c.renderer.feature_terrain);
                    render_feat_vis_select(ui, "Dynamics", &mut c.renderer.feature_dynamics);
                    render_feat_vis_select(ui, "Sky Objects", &mut c.renderer.feature_sky);
                    render_feat_vis_select(ui, "Water", &mut c.renderer.feature_water);
                    render_feat_vis_select(
                        ui,
                        "Trees/Decorators",
                        &mut c.renderer.feature_decorators,
                    );
                    render_feat_vis(ui, "⚠ Atmosphere", &mut c.renderer.feature_atmosphere);
                    render_feat_vis(ui, "⚠ Cubemaps", &mut c.renderer.feature_cubemaps);
                    render_feat_vis(
                        ui,
                        "⚠ Global Lighting",
                        &mut c.renderer.feature_global_lighting,
                    );
                    render_feat_vis(ui, "FXAA", &mut c.renderer.feature_fxaa);
                    if c.renderer.feature_fxaa {
                        render_feat_vis(ui, "FXAA Noise", &mut c.renderer.fxaa_noise);
                    }
                });

                ui.separator();
                ui.collapsing(RichText::new("Render Stages").heading(), |ui| {
                    ui.checkbox(&mut c.renderer.stage_transparent, "Transparents");
                    ui.checkbox(&mut c.renderer.stage_decals, "Decals");
                    ui.checkbox(&mut c.renderer.stage_decals_additive, "Decals (additive)");
                });

                resources
                    .get::<RendererShared>()
                    .set_render_settings(c.renderer.clone());
            })
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
            SelectionGizmoMode::Translate => GizmoMode::all_translate(),
            SelectionGizmoMode::Rotate => GizmoMode::all_rotate(),
            SelectionGizmoMode::Scale => GizmoMode::all_scale(),
        }
    }
}

fn render_feat_vis_select(ui: &mut egui::Ui, name: &str, mode: &mut RenderFeatureVisibility) {
    ui.horizontal(|ui| {
        ui.label(name);

        let rounding_l = CornerRadius {
            ne: 0,
            se: 0,
            nw: 2,
            sw: 2,
        };
        let rounding_r = CornerRadius {
            nw: 0,
            sw: 0,
            ne: 2,
            se: 2,
        };

        ui.style_mut().spacing.item_spacing = [0.0; 2].into();
        ui.style_mut().spacing.button_padding = [4.0, 0.0].into();

        ui.style_mut().visuals.widgets.active.corner_radius = rounding_l;
        ui.style_mut().visuals.widgets.hovered.corner_radius = rounding_l;
        ui.style_mut().visuals.widgets.inactive.corner_radius = rounding_l;

        if ui
            .selectable_label(
                mode.contains(RenderFeatureVisibility::VISIBLE),
                ICON_EYE.to_string(),
            )
            .clicked()
        {
            *mode ^= RenderFeatureVisibility::VISIBLE;
        }

        ui.style_mut().visuals.widgets.active.corner_radius = rounding_r;
        ui.style_mut().visuals.widgets.hovered.corner_radius = rounding_r;
        ui.style_mut().visuals.widgets.inactive.corner_radius = rounding_r;

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

        let rounding = CornerRadius {
            nw: 2,
            sw: 2,
            ne: 2,
            se: 2,
        };

        ui.style_mut().spacing.button_padding = [4.0, 0.0].into();

        ui.style_mut().visuals.widgets.active.corner_radius = rounding;
        ui.style_mut().visuals.widgets.hovered.corner_radius = rounding;
        ui.style_mut().visuals.widgets.inactive.corner_radius = rounding;

        if ui
            .selectable_label(*visible, ICON_EYE.to_string())
            .clicked()
        {
            *visible = !*visible;
        }
    });
}
