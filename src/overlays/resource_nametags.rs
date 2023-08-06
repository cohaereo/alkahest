use std::{rc::Rc, cell::RefCell};
use destiny_pkg::TagHash;
use frustum_query::frustum::Frustum;
use glam::{Mat4, Vec2, Vec4};
use imgui::{WindowFlags, Condition, ImColor32};
use winit::window::Window;

use super::{gui::OverlayProvider, camera_settings::CameraPositionOverlay};

pub struct ResourceTypeOverlay {
    pub debug_overlay: Rc<RefCell<CameraPositionOverlay>>,
    pub map: (u32, Vec<TagHash>, Vec<UnknownPoint>)
 }

 impl ResourceTypeOverlay {
    pub fn set_map_data(&mut self, size: u32, one: Vec<TagHash>, two: Vec<UnknownPoint>) {
        self.map = (size, one, two);
    }
 }

impl OverlayProvider for ResourceTypeOverlay {
    fn create_overlay(&mut self, ui: &mut imgui::Ui, window: &Window) {
        if self.debug_overlay.as_ref().borrow().show_map_resources {
            let screen_size = ui.io().display_size;
            let window_dims = window.inner_size();

            ui.window("Paint-over")
            .flags(WindowFlags::NO_BACKGROUND | WindowFlags::NO_TITLE_BAR | WindowFlags::NO_INPUTS | WindowFlags::NO_DECORATION | WindowFlags::NO_RESIZE | WindowFlags::NO_MOVE)
            .size(screen_size, Condition::Always)
            .position([0.0, 0.0], Condition::Always)
            .build(|| {
                let projection = Mat4::perspective_infinite_reverse_rh(
                    90f32.to_radians(),
                    window_dims.width as f32 / window_dims.height as f32,
                    0.0001,
                );
    
                let view = &self.debug_overlay.as_ref().borrow().camera.borrow_mut().calculate_matrix();
                let proj_view = projection.mul_mat4(view);
                let camera_frustum = Frustum::from_modelview_projection(&proj_view.to_cols_array());
    
                let draw_list = ui.get_background_draw_list();
                draw_list.with_clip_rect([0.0, 0.0], screen_size, || for unk in &mut self.map.2 {
                    if !camera_frustum.point_intersecting(&unk.position.x, &unk.position.y, &unk.position.z) {
                        continue;
                    }
    
                    let distance = unk.position.truncate().distance(self.debug_overlay.as_ref().borrow().camera.borrow().position);
                    if distance > self.debug_overlay.as_ref().borrow().map_resource_distance {
                        continue
                    }
    
                    let distance = distance / 5000.0;
    
                    let projected_point = proj_view.project_point3(unk.position.truncate());
    
                    let screen_point = Vec2::new(
                        ((projected_point.x + 1.0) * 0.5) * screen_size[0],
                        ((1.0 - projected_point.y) * 0.5) * screen_size[1]
                    );
    
                    ui.set_window_font_scale((1.0 - distance).max(0.1));
                    let c = RANDOM_COLORS[unk.resource_type as usize % 16];
                    let color = ImColor32::from_rgb(c[0], c[1], c[2]);
                    draw_list.add_circle(screen_point.to_array(), (1.0 - distance).max(0.1) * 2.0, color).filled(true).build();
                    draw_list.add_text(screen_point.to_array(), color, format!("Resource {:08x}", unk.resource_type));
                });
            });
        }

    }
}

#[derive(Clone)]
pub struct UnknownPoint {
    pub position: Vec4,
    pub resource_type: u32,
}

const RANDOM_COLORS: [[u8; 3]; 16] = [
    [0xFF, 0x00, 0x00],
    [0x00, 0xFF, 0x00],
    [0x00, 0x00, 0xFF],
    [0xFF, 0xFF, 0x00],
    [0xFF, 0x00, 0xFF],
    [0x00, 0xFF, 0xFF],
    [0x00, 0x00, 0x00],
    [0x80, 0x00, 0x00],
    [0x00, 0x80, 0x00],
    [0x00, 0x00, 0x80],
    [0x80, 0x80, 0x00],
    [0x80, 0x00, 0x80],
    [0x00, 0x80, 0x80],
    [0x80, 0x80, 0x80],
    [0xC0, 0x00, 0x00],
    [0x00, 0xC0, 0x00],
];