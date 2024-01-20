use crate::{
    camera::FpsCamera,
    ecs::{
        components::{Label, ResourceOriginType, ResourcePoint, Visible},
        resources::SelectedEntity,
        transform::Transform,
    },
    map::MapDataList,
    map_resources::MapResource,
    render::debug::DebugShapes,
    resources::Resources,
    util::text::text_color_for_background,
};

use egui::{Color32, Pos2, Rect};
use glam::Vec2;
use std::{cell::RefCell, rc::Rc, time::Instant};
use winit::window::Window;

use super::{camera_settings::CameraPositionOverlay, gui::Overlay};

pub struct ResourceTypeOverlay {
    pub debug_overlay: Rc<RefCell<CameraPositionOverlay>>,
}

impl Overlay for ResourceTypeOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &mut Resources,
        gui: &mut super::gui::GuiContext<'_>,
    ) -> bool {
        let camera = resources.get::<FpsCamera>().unwrap();
        let screen_size = ctx.screen_rect().size();
        let painter = ctx.layer_painter(egui::LayerId::background());

        {
            let mut debugshapes = resources.get_mut::<DebugShapes>().unwrap();
            for (text, point, anchor, color) in debugshapes.label_list() {
                if !camera.is_point_visible(point) {
                    continue;
                }

                let projected_point = camera.projection_view_matrix.project_point3(point);

                let screen_point = Pos2::new(
                    ((projected_point.x + 1.0) * 0.5) * screen_size.x,
                    ((1.0 - projected_point.y) * 0.5) * screen_size.y,
                );

                let color_scaled = color.0 * 255.0;
                painter.text(
                    screen_point + anchor.to_sign() * -4.,
                    anchor,
                    text,
                    egui::FontId::monospace(12.0),
                    Color32::from_rgb(
                        color_scaled.x as u8,
                        color_scaled.y as u8,
                        color_scaled.z as u8,
                    ),
                );
            }
        }

        if self.debug_overlay.borrow().show_map_resources {
            let SelectedEntity(selected_entity, block_entity_selection, _) =
                *resources.get::<SelectedEntity>().unwrap();

            let maps = resources.get::<MapDataList>().unwrap();
            if let Some((_, _, m)) = maps.current_map() {
                struct StrippedResourcePoint {
                    resource: MapResource,
                    has_havok_data: bool,
                    origin: ResourceOriginType,
                    label: Option<String>,
                }

                let mut rp_list = vec![];

                for (e, (transform, res, label, visible)) in m
                    .scene
                    .query::<(&Transform, &ResourcePoint, Option<&Label>, Option<&Visible>)>()
                    .iter()
                {
                    let distance = if selected_entity != Some(e) {
                        if !visible.map_or(true, |v| v.0) {
                            continue;
                        }

                        if !self.debug_overlay.borrow().map_resource_filter[res.resource.index()] {
                            continue;
                        }

                        if res.origin == ResourceOriginType::Map
                            && !self.debug_overlay.borrow().map_resource_show_map
                        {
                            continue;
                        }

                        if matches!(
                            res.origin,
                            ResourceOriginType::Activity | ResourceOriginType::ActivityBruteforce
                        ) && !self.debug_overlay.borrow().map_resource_show_activity
                        {
                            continue;
                        }

                        if self.debug_overlay.borrow().map_resource_only_show_named
                            && label.is_none()
                        {
                            continue;
                        }

                        let distance = transform.translation.distance(camera.position);
                        let debug_overlay = self.debug_overlay.borrow();
                        if debug_overlay.map_resource_distance_limit_enabled
                            && distance > self.debug_overlay.borrow().map_resource_distance
                        {
                            continue;
                        }

                        distance
                    } else {
                        // If the entity is selected, always sort it in front of everything else
                        0.0
                    };

                    if visible.map_or(true, |v| v.0) || selected_entity == Some(e) {
                        // Draw the debug shape before we cull the points to prevent shapes from popping in/out when the point goes off/onscreen
                        let mut debug_shapes = resources.get_mut::<DebugShapes>().unwrap();
                        res.resource.draw_debug_shape(transform, &mut debug_shapes);
                    }

                    if !camera.is_point_visible(transform.translation) {
                        continue;
                    }

                    rp_list.push((
                        e,
                        distance,
                        *transform,
                        StrippedResourcePoint {
                            resource: res.resource.clone(),
                            has_havok_data: res.has_havok_data,
                            origin: res.origin,
                            label: label.map(|v| v.0.clone()),
                        },
                    ))
                }

                rp_list.sort_by(|a, b| a.1.total_cmp(&b.1));

                if !block_entity_selection {
                    if let Some(mouse_event) = ctx.input(|i| {
                        i.events
                            .iter()
                            .find(|e| matches!(e, egui::Event::PointerButton { .. }))
                            .cloned()
                    }) {
                        let egui::Event::PointerButton {
                            pos,
                            button,
                            pressed,
                            modifiers,
                        } = mouse_event
                        else {
                            unreachable!();
                        };

                        if pressed
                            && button == egui::PointerButton::Secondary
                            && modifiers.is_none()
                        {
                            for (e, _, transform, res) in &rp_list {
                                if selected_entity == Some(*e) {
                                    continue;
                                }

                                let projected_point = camera
                                    .projection_view_matrix
                                    .project_point3(transform.translation);

                                let screen_point = Vec2::new(
                                    ((projected_point.x + 1.0) * 0.5) * screen_size.x,
                                    ((1.0 - projected_point.y) * 0.5) * screen_size.y,
                                );

                                let select_rect =
                                    if self.debug_overlay.borrow().show_map_resource_label {
                                        let debug_string = res.resource.debug_string();
                                        let debug_string = if let Some(l) = &res.label {
                                            format!("{l}\n{debug_string}")
                                        } else {
                                            debug_string
                                        };

                                        let debug_string_font = egui::FontId::proportional(14.0);
                                        let debug_string_pos: egui::Pos2 =
                                            (screen_point + Vec2::new(14.0, 0.0)).to_array().into();

                                        let debug_string_galley = painter.layout_no_wrap(
                                            debug_string.clone(),
                                            debug_string_font.clone(),
                                            Color32::WHITE,
                                        );

                                        let mut debug_string_rect = egui::Align2::LEFT_CENTER
                                            .anchor_rect(Rect::from_min_size(
                                                debug_string_pos,
                                                debug_string_galley.size(),
                                            ));
                                        debug_string_rect
                                            .extend_with_x(debug_string_pos.x - 11.0 - 14.0);

                                        debug_string_rect
                                    } else {
                                        egui::Align2::CENTER_CENTER.anchor_rect(
                                            Rect::from_min_size(
                                                screen_point.to_array().into(),
                                                egui::vec2(22.0, 22.0),
                                            ),
                                        )
                                    };

                                if select_rect.contains(pos) {
                                    *resources.get_mut::<SelectedEntity>().unwrap() =
                                        SelectedEntity(Some(*e), true, Instant::now());
                                    break;
                                }
                            }
                        }
                    }
                }

                rp_list.reverse();

                for (e, _, transform, res) in rp_list {
                    let projected_point = camera
                        .projection_view_matrix
                        .project_point3(transform.translation);

                    let screen_point = Vec2::new(
                        ((projected_point.x + 1.0) * 0.5) * screen_size.x,
                        ((1.0 - projected_point.y) * 0.5) * screen_size.y,
                    );

                    let c = res.resource.debug_color();
                    let color = egui::Color32::from_rgb(c[0], c[1], c[2]);
                    if self.debug_overlay.borrow().show_map_resource_label
                        || selected_entity == Some(e)
                    {
                        let debug_string = res.resource.debug_string();
                        let debug_string = if let Some(l) = res.label {
                            format!("{l}\n{debug_string}")
                        } else {
                            debug_string
                        };

                        let debug_string_font = egui::FontId::proportional(14.0);
                        let debug_string_pos: egui::Pos2 =
                            (screen_point + Vec2::new(14.0, 0.0)).to_array().into();

                        let debug_string_galley = painter.layout_no_wrap(
                            debug_string.clone(),
                            debug_string_font.clone(),
                            Color32::WHITE,
                        );

                        let mut debug_string_rect = egui::Align2::LEFT_CENTER.anchor_rect(
                            Rect::from_min_size(debug_string_pos, debug_string_galley.size()),
                        );
                        debug_string_rect.extend_with_x(debug_string_pos.x - 11.0 - 14.0);

                        if selected_entity == Some(e) {
                            painter.rect(
                                debug_string_rect.expand(8.0),
                                egui::Rounding::same(4.0),
                                Color32::TRANSPARENT,
                                egui::Stroke::new(3.0, Color32::from_rgb(255, 150, 50)),
                            );
                        }

                        if self.debug_overlay.borrow().map_resource_label_background {
                            let background_color = text_color_for_background(color);
                            let white_bg = background_color.r() == 255;
                            painter.rect(
                                debug_string_rect.expand(4.0),
                                egui::Rounding::none(),
                                if white_bg {
                                    Color32::from_white_alpha(196)
                                } else {
                                    Color32::from_black_alpha(128)
                                },
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
                            gui.icons.icon_havok.id(),
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
                                ResourceOriginType::ActivityBruteforce => "Ab",
                                ResourceOriginType::Ambient => "AM",
                            },
                            egui::FontId::monospace(12.0),
                            match res.origin {
                                ResourceOriginType::Map => Color32::LIGHT_RED,
                                ResourceOriginType::Activity => Color32::GREEN,
                                ResourceOriginType::ActivityBruteforce => Color32::RED,
                                ResourceOriginType::Ambient => Color32::from_rgb(0, 255, 255),
                            },
                        );
                    }
                }
            }
        }

        true
    }
}
