use crate::{
    camera::FpsCamera,
    ecs::{
        components::{ResourceOriginType, ResourcePoint},
        transform::Transform,
    },
    map::MapDataList,
    map_resources::MapResource,
    render::debug::DebugShapes,
    resources::Resources,
};

use egui::{Color32, Rect};
use frustum_query::frustum::Frustum;
use glam::{Mat4, Vec2};
use std::{cell::RefCell, rc::Rc};
use winit::window::Window;

use super::{
    camera_settings::CameraPositionOverlay,
    gui::{GuiResources, OverlayProvider},
};

pub struct ResourceTypeOverlay {
    pub debug_overlay: Rc<RefCell<CameraPositionOverlay>>,
}

impl OverlayProvider for ResourceTypeOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &mut Resources,
        icons: &GuiResources,
    ) {
        if self.debug_overlay.borrow().show_map_resources {
            let screen_size = ctx.screen_rect().size();

            let painter = ctx.layer_painter(egui::LayerId::background());

            let projection = Mat4::perspective_infinite_reverse_rh(
                90f32.to_radians(),
                screen_size.x / screen_size.y,
                0.0001,
            );

            let camera = resources.get::<FpsCamera>().unwrap();
            let view = camera.calculate_matrix();
            let proj_view = projection.mul_mat4(&view);
            let camera_frustum = Frustum::from_modelview_projection(&proj_view.to_cols_array());

            let maps = resources.get::<MapDataList>().unwrap();
            if let Some((_, _, m)) = maps.current_map() {
                struct StrippedResourcePoint {
                    resource: MapResource,
                    has_havok_data: bool,
                    origin: ResourceOriginType,
                }

                let mut rp_list = vec![];

                for (_, (transform, res)) in m.scene.query::<(&Transform, &ResourcePoint)>().iter()
                {
                    if !self.debug_overlay.borrow().map_resource_filter
                        [res.resource.index() as usize]
                    {
                        continue;
                    }

                    let distance = transform.translation.distance(camera.position);
                    if distance > self.debug_overlay.borrow().map_resource_distance {
                        continue;
                    }

                    // Draw the debug shape before we cull the points to prevent shapes from popping in/out when the point goes off/onscreen
                    let mut debug_shapes = resources.get_mut::<DebugShapes>().unwrap();
                    res.resource.draw_debug_shape(
                        transform.translation,
                        transform.rotation,
                        &mut debug_shapes,
                    );

                    if !camera_frustum.point_intersecting(
                        &transform.translation.x,
                        &transform.translation.y,
                        &transform.translation.z,
                    ) {
                        continue;
                    }

                    rp_list.push((
                        distance,
                        *transform,
                        StrippedResourcePoint {
                            resource: res.resource.clone(),
                            has_havok_data: res.has_havok_data,
                            origin: res.origin,
                        },
                    ))
                }

                if self.debug_overlay.borrow().map_resource_label_background {
                    rp_list.sort_by(|a, b| a.0.total_cmp(&b.0));
                    rp_list.reverse();
                }

                for (_, transform, res) in rp_list {
                    let projected_point = proj_view.project_point3(transform.translation);

                    let screen_point = Vec2::new(
                        ((projected_point.x + 1.0) * 0.5) * screen_size.x,
                        ((1.0 - projected_point.y) * 0.5) * screen_size.y,
                    );

                    let c = res.resource.debug_color();
                    let color = egui::Color32::from_rgb(c[0], c[1], c[2]);
                    if self.debug_overlay.borrow().show_map_resource_label {
                        let debug_string = res.resource.debug_string();
                        let debug_string_font = egui::FontId::proportional(14.0);
                        let debug_string_pos: egui::Pos2 =
                            (screen_point + Vec2::new(14.0, 0.0)).to_array().into();
                        if self.debug_overlay.borrow().map_resource_label_background {
                            let debug_string_galley = painter.layout_no_wrap(
                                debug_string.clone(),
                                debug_string_font.clone(),
                                Color32::WHITE,
                            );
                            let mut debug_string_rect = egui::Align2::LEFT_CENTER.anchor_rect(
                                Rect::from_min_size(debug_string_pos, debug_string_galley.size()),
                            );
                            debug_string_rect.extend_with_x(debug_string_pos.x - 11.0 - 14.0);

                            painter.rect(
                                debug_string_rect,
                                egui::Rounding::none(),
                                Color32::from_black_alpha(128),
                                egui::Stroke::default(),
                            );
                        }

                        painter.text(
                            debug_string_pos,
                            egui::Align2::LEFT_CENTER,
                            debug_string,
                            debug_string_font,
                            color,
                        );
                    }

                    painter.text(
                        screen_point.to_array().into(),
                        egui::Align2::CENTER_CENTER,
                        res.resource.debug_icon().to_string(),
                        egui::FontId::proportional(22.0),
                        color,
                    );

                    if res.has_havok_data {
                        painter.image(
                            icons.icon_havok.id(),
                            egui::Rect::from_center_size(
                                egui::Pos2::from(screen_point.to_array())
                                    - egui::pos2(12., 12.).to_vec2(),
                                egui::vec2(16.0, 16.0),
                            ),
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }

                    if res.origin != ResourceOriginType::Map {
                        painter.rect(
                            egui::Rect::from_min_size(
                                screen_point.to_array().into(),
                                [11.0, 11.0].into(),
                            ),
                            egui::Rounding::none(),
                            Color32::from_black_alpha(152),
                            egui::Stroke::default(),
                        );

                        painter.text(
                            egui::Pos2::from(screen_point.to_array()) + egui::vec2(5.5, 5.5),
                            egui::Align2::CENTER_CENTER,
                            match res.origin {
                                ResourceOriginType::Map => "M",
                                ResourceOriginType::Activity => "A",
                                ResourceOriginType::Activity2 => "A2",
                            },
                            egui::FontId::monospace(12.0),
                            match res.origin {
                                ResourceOriginType::Map => Color32::LIGHT_RED,
                                ResourceOriginType::Activity => Color32::GREEN,
                                ResourceOriginType::Activity2 => Color32::RED,
                            },
                        );
                    }
                }
            }
        }
    }
}
