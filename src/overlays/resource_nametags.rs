use crate::{
    camera::FpsCamera, map::MapDataList, render::debug::DebugShapes, resources::Resources,
};

use frustum_query::frustum::Frustum;
use glam::{Mat4, Vec2};
use std::{cell::RefCell, rc::Rc};
use winit::window::Window;

use super::{camera_settings::CameraPositionOverlay, gui::OverlayProvider};

pub struct ResourceTypeOverlay {
    pub debug_overlay: Rc<RefCell<CameraPositionOverlay>>,
}

impl OverlayProvider for ResourceTypeOverlay {
    fn draw(&mut self, ctx: &egui::Context, window: &Window, resources: &mut Resources) {
        if self.debug_overlay.borrow().show_map_resources {
            let screen_size = window.inner_size();
            let window_dims = window.inner_size();

            let painter = ctx.layer_painter(egui::LayerId::background());

            let projection = Mat4::perspective_infinite_reverse_rh(
                90f32.to_radians(),
                window_dims.width as f32 / window_dims.height as f32,
                0.0001,
            );

            let mut camera = resources.get_mut::<FpsCamera>().unwrap();
            let view = camera.calculate_matrix();
            let proj_view = projection.mul_mat4(&view);
            let camera_frustum = Frustum::from_modelview_projection(&proj_view.to_cols_array());

            let maps = resources.get::<MapDataList>().unwrap();
            if let Some(m) = maps.current_map() {
                for (res, _) in m.resource_points.iter() {
                    if !self.debug_overlay.borrow().map_resource_filter
                        [res.resource.index() as usize]
                    {
                        continue;
                    }

                    let distance = res.translation.truncate().distance(camera.position);
                    if distance > self.debug_overlay.borrow().map_resource_distance {
                        continue;
                    }

                    // Draw the debug shape before we cull the points to prevent shapes from popping in/out when the point goes off/onscreen
                    let mut debug_shapes = resources.get_mut::<DebugShapes>().unwrap();
                    res.resource
                        .draw_debug_shape(res.translation, res.rotation, &mut debug_shapes);

                    if !camera_frustum.point_intersecting(
                        &res.translation.x,
                        &res.translation.y,
                        &res.translation.z,
                    ) {
                        continue;
                    }

                    let projected_point = proj_view.project_point3(res.translation.truncate());

                    let screen_point = Vec2::new(
                        ((projected_point.x + 1.0) * 0.5) * screen_size.width as f32,
                        ((1.0 - projected_point.y) * 0.5) * screen_size.height as f32,
                    );

                    let c = res.resource.debug_color();
                    let color = egui::Color32::from_rgb(c[0], c[1], c[2]);
                    painter.text(
                        screen_point.to_array().into(),
                        egui::Align2::CENTER_CENTER,
                        res.resource.debug_icon().to_string(),
                        egui::FontId::proportional(22.0),
                        color,
                    );

                    if self.debug_overlay.borrow().show_map_resource_label {
                        painter.text(
                            (screen_point + Vec2::new(12.0, 0.0)).to_array().into(),
                            egui::Align2::LEFT_CENTER,
                            res.resource.debug_string(),
                            egui::FontId::proportional(14.0),
                            color,
                        );
                    }
                }
            }
        }
    }
}
