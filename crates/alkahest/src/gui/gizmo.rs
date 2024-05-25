use alkahest_renderer::{
    camera::Camera,
    ecs::{render::update_entity_transform, resources::SelectedEntity, transform::Transform},
    renderer::{Renderer, RendererShared},
    resources::Resources,
};
use egui::{epaint::Vertex, Mesh, PointerButton, Pos2, Rect, Rgba};
use glam::{DQuat, DVec3};
use transform_gizmo_egui::{
    enum_set, math::Transform as GTransform, Gizmo, GizmoConfig, GizmoInteraction, GizmoMode,
    GizmoOrientation, GizmoResult,
};

use crate::maplist::MapList;

pub fn draw_transform_gizmos(renderer: &Renderer, ctx: &egui::Context, resources: &Resources) {
    let Some(selected) = resources.get::<SelectedEntity>().selected() else {
        return;
    };

    let maplist = resources.get::<MapList>();
    if let Some(map) = maplist.current_map() {
        let Ok(mut transform) = map.scene.get::<&mut Transform>(selected) else {
            return;
        };
        let camera = resources.get::<Camera>();

        let mut gizmo = resources.get_mut::<Gizmo>();
        let old_config = *gizmo.config();
        gizmo.update_config(GizmoConfig {
            view_matrix: camera.world_to_camera.as_dmat4().into(),
            projection_matrix: camera.camera_to_projective.as_dmat4().into(),
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
            update_entity_transform(&map.scene, selected);
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
