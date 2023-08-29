use crate::{
    camera::FpsCamera, map::MapDataList, map_resources::MapResource, resources::Resources,
};
use destiny_pkg::TagHash;
use frustum_query::frustum::Frustum;
use glam::{Mat4, Quat, Vec2, Vec4};
use imgui::{Condition, ImColor32, WindowFlags};
use std::{cell::RefCell, rc::Rc};
use winit::window::Window;

use super::{camera_settings::CameraPositionOverlay, gui::OverlayProvider};

pub struct ResourceTypeOverlay {
    pub debug_overlay: Rc<RefCell<CameraPositionOverlay>>,
}

impl OverlayProvider for ResourceTypeOverlay {
    fn create_overlay(&mut self, ui: &mut imgui::Ui, window: &Window, resources: &mut Resources) {
        if self.debug_overlay.borrow().show_map_resources {
            let screen_size = ui.io().display_size;
            let window_dims = window.inner_size();

            ui.window("Paint-over")
                .flags(
                    WindowFlags::NO_BACKGROUND
                        | WindowFlags::NO_TITLE_BAR
                        | WindowFlags::NO_INPUTS
                        | WindowFlags::NO_DECORATION
                        | WindowFlags::NO_RESIZE
                        | WindowFlags::NO_MOVE,
                )
                .save_settings(false)
                .size(screen_size, Condition::Always)
                .position([0.0, 0.0], Condition::Always)
                .build(|| {
                    let projection = Mat4::perspective_infinite_reverse_rh(
                        90f32.to_radians(),
                        window_dims.width as f32 / window_dims.height as f32,
                        0.0001,
                    );

                    let mut camera = resources.get_mut::<FpsCamera>().unwrap();
                    let view = camera.calculate_matrix();
                    let proj_view = projection.mul_mat4(&view);
                    let camera_frustum =
                        Frustum::from_modelview_projection(&proj_view.to_cols_array());

                    let draw_list = ui.get_background_draw_list();
                    draw_list.with_clip_rect([0.0, 0.0], screen_size, || {
                        let maps = resources.get::<MapDataList>().unwrap();
                        if let Some(m) = maps.current_map() {
                            for (res, _) in m.resource_points.iter() {
                                if !camera_frustum.point_intersecting(
                                    &res.translation.x,
                                    &res.translation.y,
                                    &res.translation.z,
                                ) {
                                    continue;
                                }

                                let distance = res.translation.truncate().distance(camera.position);
                                if distance > self.debug_overlay.borrow().map_resource_distance {
                                    continue;
                                }

                                let projected_point =
                                    proj_view.project_point3(res.translation.truncate());

                                let screen_point = Vec2::new(
                                    ((projected_point.x + 1.0) * 0.5) * screen_size[0],
                                    ((1.0 - projected_point.y) * 0.5) * screen_size[1],
                                );

                                if !self.debug_overlay.borrow().map_resource_filter
                                    [res.resource.index() as usize]
                                {
                                    continue;
                                }

                                let c = res.resource.debug_color();
                                let color = ImColor32::from_rgb(c[0], c[1], c[2]);
                                ui.set_window_font_scale(1.25);
                                draw_list.add_text(
                                    screen_point.to_array(),
                                    color,
                                    res.resource.debug_icon().to_string(),
                                );

                                ui.set_window_font_scale(1.0);
                                if self.debug_overlay.borrow().show_map_resource_label {
                                    draw_list.add_text(
                                        (screen_point + Vec2::new(22.0, 0.0)).to_array(),
                                        color,
                                        res.resource.debug_string(),
                                    );

                                    if res.entity.is_valid() && res.resource.is_unknown() {
                                        let offset = ui.calc_text_size(res.resource.debug_string());
                                        draw_list.add_text(
                                            (screen_point + Vec2::new(22.0 + offset[0], 0.0))
                                                .to_array(),
                                            ImColor32::WHITE,
                                            format!(" (ent {})", res.entity),
                                        );
                                    }
                                }
                            }
                        }
                    });
                });
        }
    }
}

#[derive(Clone)]
pub struct ResourcePoint {
    pub translation: Vec4,
    pub rotation: Quat,
    pub entity: TagHash,
    pub resource_type: u32,
    pub resource: MapResource,
}
