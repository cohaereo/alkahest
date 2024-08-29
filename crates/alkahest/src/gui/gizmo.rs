use alkahest_renderer::{
    camera::Camera,
    ecs::{resources::SelectedEntity, transform::Transform},
    icons::{ICON_AXIS_ARROW, ICON_CURSOR_DEFAULT, ICON_RESIZE, ICON_ROTATE_ORBIT},
    renderer::Renderer,
    resources::AppResources,
};
use egui::{
    epaint::Vertex, Context, LayerId, Mesh, PointerButton, Pos2, Rgba, RichText, Rounding, Ui,
    UiStackInfo,
};
use glam::{DQuat, DVec3};
use transform_gizmo_egui::{
    math::Transform as GTransform, Gizmo, GizmoConfig, GizmoInteraction, GizmoResult,
};
use winit::window::Window;

use crate::{
    gui::{
        configuration::SelectionGizmoMode,
        context::{GuiCtx, GuiView, ViewResult},
    },
    maplist::MapList,
};

pub struct GizmoSelector;

impl GuiView for GizmoSelector {
    fn draw(
        &mut self,
        ctx: &Context,
        _window: &Window,
        resources: &AppResources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        let mut ui = Ui::new(
            ctx.clone(),
            LayerId::background(),
            "gizmo_selector_overlay".into(),
            ctx.available_rect().shrink(16.0),
            ctx.screen_rect(),
            UiStackInfo::default(),
        );

        ui.horizontal(|ui| {
            let mut gizmo_mode = resources.get_mut::<SelectionGizmoMode>();
            let rounding_l = Rounding {
                ne: 0.0,
                se: 0.0,
                nw: 4.0,
                sw: 4.0,
            };
            let rounding_m = Rounding {
                nw: 0.0,
                sw: 0.0,
                ne: 0.0,
                se: 0.0,
            };
            let rounding_r = Rounding {
                nw: 0.0,
                sw: 0.0,
                ne: 4.0,
                se: 4.0,
            };

            ui.style_mut().spacing.item_spacing = [0.0; 2].into();
            ui.style_mut().spacing.button_padding = [4.0, 4.0].into();

            ui.style_mut().visuals.widgets.active.rounding = rounding_l;
            ui.style_mut().visuals.widgets.hovered.rounding = rounding_l;
            ui.style_mut().visuals.widgets.inactive.rounding = rounding_l;

            ui.selectable_value(
                &mut *gizmo_mode,
                SelectionGizmoMode::Select,
                RichText::new(ICON_CURSOR_DEFAULT.to_string()).size(16.0),
            )
            .on_hover_text("Hotkey: 1");

            ui.style_mut().visuals.widgets.active.rounding = rounding_m;
            ui.style_mut().visuals.widgets.hovered.rounding = rounding_m;
            ui.style_mut().visuals.widgets.inactive.rounding = rounding_m;

            ui.selectable_value(
                &mut *gizmo_mode,
                SelectionGizmoMode::Translate,
                RichText::new(ICON_AXIS_ARROW.to_string()).size(16.0),
            )
            .on_hover_text("Hotkey: 2");

            ui.selectable_value(
                &mut *gizmo_mode,
                SelectionGizmoMode::Rotate,
                RichText::new(ICON_ROTATE_ORBIT.to_string()).size(16.0),
            )
            .on_hover_text("Hotkey: 3");

            ui.style_mut().visuals.widgets.active.rounding = rounding_r;
            ui.style_mut().visuals.widgets.hovered.rounding = rounding_r;
            ui.style_mut().visuals.widgets.inactive.rounding = rounding_r;

            ui.selectable_value(
                &mut *gizmo_mode,
                SelectionGizmoMode::Scale,
                RichText::new(ICON_RESIZE.to_string()).size(16.0),
            )
            .on_hover_text("Hotkey: 4");
        });

        None
    }
}

pub fn draw_transform_gizmos(renderer: &Renderer, ctx: &egui::Context, resources: &AppResources) {
    let Some(selected) = resources.get::<SelectedEntity>().selected() else {
        return;
    };

    let mut gizmo_mode = resources.get_mut::<SelectionGizmoMode>();
    if ctx.input(|i| i.key_pressed(egui::Key::Num1)) {
        *gizmo_mode = SelectionGizmoMode::Select;
    } else if ctx.input(|i| i.key_pressed(egui::Key::Num2)) {
        *gizmo_mode = SelectionGizmoMode::Translate;
    } else if ctx.input(|i| i.key_pressed(egui::Key::Num3)) {
        *gizmo_mode = SelectionGizmoMode::Rotate;
    } else if ctx.input(|i| i.key_pressed(egui::Key::Num4)) {
        *gizmo_mode = SelectionGizmoMode::Scale;
    }

    let mut maplist = resources.get_mut::<MapList>();
    if let Some(map) = maplist.current_map_mut() {
        let Some(mut transform) = map.scene.get_mut::<Transform>(selected) else {
            return;
        };
        let camera = resources.get::<Camera>();

        let mut gizmo = resources.get_mut::<Gizmo>();
        let old_config = *gizmo.config();
        gizmo.update_config(GizmoConfig {
            view_matrix: camera.world_to_camera.as_dmat4().into(),
            projection_matrix: camera.camera_to_projective.as_dmat4().into(),
            modes: gizmo_mode.to_enumset(),
            ..old_config
        });

        if let Some((_result, new_transform)) = gizmo_interact(
            &mut gizmo,
            ctx,
            &[GTransform {
                scale: transform.scale.as_dvec3().into(),
                rotation: transform.rotation.as_dquat().into(),
                translation: transform.translation.as_dvec3().into(),
            }],
        ) {
            renderer.pickbuffer.cancel_request();
            transform.translation = DVec3::from(new_transform[0].translation).as_vec3();
            transform.rotation = DQuat::from(new_transform[0].rotation).as_quat().normalize();
            transform.scale = DVec3::from(new_transform[0].scale).as_vec3();
        }
    }
}

#[must_use]
fn gizmo_interact(
    gizmo: &mut Gizmo,
    ctx: &egui::Context,
    targets: &[GTransform],
) -> Option<(GizmoResult, Vec<GTransform>)> {
    let cursor_pos = ctx
        .input(|input| input.pointer.hover_pos())
        .unwrap_or_default();

    let mut viewport = gizmo.config().viewport;
    if !viewport.is_finite() {
        viewport = ctx.screen_rect();
    }

    gizmo.update_config(GizmoConfig {
        viewport,
        pixels_per_point: ctx.pixels_per_point(),
        ..*gizmo.config()
    });

    let gizmo_result = gizmo.update(
        GizmoInteraction {
            cursor_pos: (cursor_pos.x, cursor_pos.y),
            drag_started: ctx.input(|input| input.pointer.button_pressed(PointerButton::Primary)),
            dragging: ctx.input(|input| input.pointer.button_down(PointerButton::Primary)),
        },
        targets,
    );

    let draw_data = gizmo.draw();

    ctx.layer_painter(egui::LayerId::background())
        // .with_clip_rect(egui_viewport)
        .add(Mesh {
            indices: draw_data.indices,
            vertices: draw_data
                .vertices
                .into_iter()
                .zip(draw_data.colors)
                .map(|(pos, [r, g, b, a])| Vertex {
                    pos: pos.into(),
                    uv: Pos2::default(),
                    color: Rgba::from_rgba_premultiplied(r, g, b, a).into(),
                })
                .collect(),
            ..Default::default()
        });

    gizmo_result
}
