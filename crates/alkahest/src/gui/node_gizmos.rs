use alkahest_renderer::{
    camera::Camera,
    ecs::{
        common::{Hidden, Icon, Label, ResourceOrigin},
        resources::SelectedEntity,
        tags::{NodeFilter, NodeFilterSet},
        transform::Transform,
    },
    icons::ICON_HELP,
    renderer::RendererShared,
    resources::Resources,
    ColorExt,
};
use egui::{Color32, Context, Frame, Pos2, Rect, Sense, Ui};
use glam::Vec2;
use winit::window::Window;

use crate::{
    config,
    gui::context::{GuiCtx, GuiView, ViewResult},
    maplist::MapList,
};

pub struct NodeGizmoOverlay;

impl GuiView for NodeGizmoOverlay {
    fn draw(
        &mut self,
        ctx: &Context,
        _window: &Window,
        resources: &Resources,
        gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
        if config::with(|c| !c.visual.node_nametags) {
            return None;
        }

        let camera = resources.get::<Camera>();
        let screen_size = ctx.screen_rect().size();
        let painter = ctx.layer_painter(egui::LayerId::background());

        // egui::CentralPanel::default()
        //     .frame(Frame::default())
        //     .show(ctx, |ui| {

        let mut panel_ui = Ui::new(
            ctx.clone(),
            egui::LayerId::background(),
            "node_nametags".into(),
            ctx.available_rect(),
            ctx.screen_rect(),
        );

        let mut selected_entity = resources.get_mut::<SelectedEntity>();
        let mut top_hovered = None;
        let mut rp_list = vec![];
        let response = panel_ui.allocate_response(panel_ui.available_size(), Sense::click());
        // let response = panel_ui.allocate_response(egui::vec2(4.0, 4.0), Sense::click());

        // {
        //     let mut debugshapes = resources.get_mut::<DebugShapes>().unwrap();
        //     for (text, point, anchor, color) in debugshapes.label_list() {
        //         if !camera.is_point_visible(point) {
        //             continue;
        //         }
        //
        //         let projected_point = camera.projection_view_matrix.project_point3(point);
        //
        //         let screen_point = Pos2::new(
        //             ((projected_point.x + 1.0) * 0.5) * screen_size.x,
        //             ((1.0 - projected_point.y) * 0.5) * screen_size.y,
        //         );
        //
        //         let color_scaled = color.0 * 255.0;
        //         painter.text(
        //             screen_point + anchor.to_sign() * -4.,
        //             anchor,
        //             text,
        //             egui::FontId::monospace(12.0),
        //             Color32::from_rgb(
        //                 color_scaled.x as u8,
        //                 color_scaled.y as u8,
        //                 color_scaled.z as u8,
        //             ),
        //         );
        //     }
        // }

        // if self.debug_overlay.borrow().show_map_resources {
        if true {
            let maps = resources.get::<MapList>();
            if let Some(map) = maps.current_map() {
                struct NodeDisplayPoint {
                    has_havok_data: bool,
                    origin: Option<ResourceOrigin>,
                    label: String,
                    icon: Option<Icon>,
                }

                let filters = resources.get::<NodeFilterSet>();
                for (e, (transform, origin, label, icon, filter)) in map
                    .scene
                    .query::<(
                        &Transform,
                        Option<&ResourceOrigin>,
                        Option<&Label>,
                        Option<&Icon>,
                        Option<&NodeFilter>,
                    )>()
                    .without::<&Hidden>()
                    .iter()
                {
                    if let Some(filter) = filter {
                        if !filters.contains(filter) {
                            continue;
                        }
                    } else {
                        if !filters.contains(&NodeFilter::Unknown) {
                            continue;
                        }
                    }

                    let distance = if selected_entity.selected() != Some(e) {
                        // if !visible.map_or(true, |v| v.0) {
                        //     continue;
                        // }

                        // if !self.debug_overlay.borrow().map_resource_filter[res.resource.index()] {
                        //     continue;
                        // }
                        //
                        // if res.origin == ResourceOrigin::Map
                        //     && !self.debug_overlay.borrow().map_resource_show_map
                        // {
                        //     continue;
                        // }
                        //
                        // if matches!(
                        //     res.origin,
                        //     ResourceOrigin::Activity | ResourceOrigin::ActivityBruteforce
                        // ) && !self.debug_overlay.borrow().map_resource_show_activity
                        // {
                        //     continue;
                        // }
                        //
                        // if self.debug_overlay.borrow().map_resource_only_show_named
                        //     && label.is_none()
                        // {
                        //     continue;
                        // }

                        transform.translation.distance(camera.position())
                        // let debug_overlay = self.debug_overlay.borrow();
                        // if debug_overlay.map_resource_distance_limit_enabled
                        //     && distance > self.debug_overlay.borrow().map_resource_distance
                        // {
                        //     continue;
                        // }
                    } else {
                        // If the entity is selected, always sort it in front of everything else
                        0.0
                    };

                    // if visible.map_or(true, |v| v.0) || selected_entity == Some(e) {
                    //     // Draw the debug shape before we cull the points to prevent shapes from popping in/out when the point goes off/onscreen
                    //     let mut debug_shapes = resources.get_mut::<DebugShapes>().unwrap();
                    //     res.resource.draw_debug_shape(transform, &mut debug_shapes);
                    // }

                    if !camera.is_point_visible(transform.translation) {
                        continue;
                    }

                    rp_list.push((
                        e,
                        distance,
                        *transform,
                        NodeDisplayPoint {
                            has_havok_data: false,
                            origin: origin.cloned(),
                            label: label.map(|v| v.label.clone()).unwrap_or_default(),
                            icon: icon.cloned(),
                        }, // StrippedResourcePoint {
                           //     resource: res.resource.clone(),
                           //     has_havok_data: res.has_havok_data,
                           //     origin: res.origin,
                           //     label: label.map(|v| v.0.clone()),
                           // },
                    ))
                }

                rp_list.sort_by(|a, b| a.1.total_cmp(&b.1));

                rp_list.reverse();

                for (i, (e, _, transform, node)) in rp_list.iter().enumerate() {
                    let projected_point = camera
                        .world_to_projective
                        .project_point3(transform.translation);

                    let screen_point = Vec2::new(
                        ((projected_point.x + 1.0) * 0.5) * screen_size.x,
                        ((1.0 - projected_point.y) * 0.5) * screen_size.y,
                    );

                    let icon = node.icon.clone().unwrap_or(Icon::Unicode(ICON_HELP));
                    // let c = res.resource.debug_color();
                    // let color = egui::Color32::from_rgb(c[0], c[1], c[2]);
                    let color = icon.color();
                    // if self.debug_overlay.borrow().show_map_resource_label
                    //     || selected_entity == Some(e)
                    if true {
                        // let debug_string = res.resource.debug_string();
                        // let debug_string = if let Some(l) = res.label {
                        //     format!("{l}\n{debug_string}")
                        // } else {
                        //     debug_string
                        // };
                        let debug_string = &node.label;

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

                        if selected_entity.selected() == Some(*e) {
                            painter.rect(
                                debug_string_rect.expand(8.0),
                                egui::Rounding::same(4.0),
                                Color32::TRANSPARENT,
                                egui::Stroke::new(
                                    3.0,
                                    Color32::from_rgba_unmultiplied(255, 150, 50, 255),
                                ),
                            );
                        }

                        if response.hovered() {
                            if let Some(mouse_pos) = ctx.input(|i| i.pointer.latest_pos()) {
                                if debug_string_rect.expand(4.0).contains(mouse_pos) {
                                    top_hovered = Some((i, debug_string_rect));
                                }
                            }
                        }

                        // if self.debug_overlay.borrow().map_resource_label_background {
                        let background_color = color.text_color_for_background();
                        let white_bg = background_color.r() == 255;
                        painter.rect(
                            debug_string_rect.expand(4.0),
                            egui::Rounding::ZERO,
                            if white_bg {
                                Color32::from_white_alpha(128)
                            } else {
                                Color32::from_black_alpha(96)
                            },
                            egui::Stroke::default(),
                        );
                        // }

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
                        icon.to_string(),
                        egui::FontId::proportional(22.0),
                        color,
                    );

                    if node.has_havok_data {
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

                    if node.origin != Some(ResourceOrigin::Map) {
                        painter.rect(
                            egui::Rect::from_min_size(
                                screen_point.to_array().into(),
                                [11.0, 11.0].into(),
                            ),
                            egui::Rounding::ZERO,
                            Color32::from_black_alpha(152),
                            egui::Stroke::default(),
                        );

                        if let Some(origin) = node.origin {
                            painter.text(
                                egui::Pos2::from(screen_point.to_array()) + egui::vec2(5.5, 5.5),
                                egui::Align2::CENTER_CENTER,
                                match origin {
                                    ResourceOrigin::Map => "M",
                                    ResourceOrigin::Activity => "A",
                                    ResourceOrigin::ActivityBruteforce => "Ab",
                                    ResourceOrigin::Ambient => "AM",
                                },
                                egui::FontId::monospace(12.0),
                                match origin {
                                    ResourceOrigin::Map => Color32::LIGHT_RED,
                                    ResourceOrigin::Activity => Color32::GREEN,
                                    ResourceOrigin::ActivityBruteforce => Color32::RED,
                                    ResourceOrigin::Ambient => Color32::from_rgb(0, 255, 255),
                                },
                            );
                        }
                    }
                }

                if let Some((_top_index, top_rect)) = top_hovered {
                    // let is_hovered = if egui_lmb_clicked(ctx).is_some() {
                    //     selected_entity.select(rp_list[top_index].0);
                    //     false
                    // } else {
                    //     selected_entity.selected() != Some(rp_list[top_index].0)
                    // };
                    let is_hovered = true;

                    painter.rect(
                        top_rect.expand(8.0),
                        egui::Rounding::same(4.0),
                        Color32::TRANSPARENT,
                        egui::Stroke::new(
                            3.0,
                            Color32::from_rgba_unmultiplied(
                                255,
                                150,
                                50,
                                if is_hovered { 150 } else { 255 },
                            ),
                        ),
                    );
                }
            }
        }

        if response.clicked() {
            if let Some((top_index, _top_rect)) = top_hovered {
                selected_entity.select(rp_list[top_index].0);
            } else {
                if let Some(mouse_pos) = ctx.pointer_interact_pos() {
                    let renderer = resources.get::<RendererShared>();
                    renderer.pickbuffer.request_selection(
                        (mouse_pos.x * ctx.pixels_per_point()).round() as u32,
                        (mouse_pos.y * ctx.pixels_per_point()).round() as u32,
                    );
                }
            }
        }
        // });

        None
    }
}

fn egui_lmb_clicked(ctx: &Context) -> Option<Pos2> {
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

        if pressed && button == egui::PointerButton::Primary && modifiers.is_none() {
            return Some(pos);
        }
    }

    None
}
