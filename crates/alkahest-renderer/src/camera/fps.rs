use glam::{Mat4, Quat, Vec2, Vec3, Vec3Swizzles};

use super::{tween::Tween, CameraController};
use crate::{input::Key, util::Vec3Ext};

pub struct FpsCamera {
    pub orientation: Vec2,
    pub rotation: Quat,
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub position: Vec3,
    target_position: Vec3,
    pub speed_mul: f32,
}

impl FpsCamera {
    fn update_vectors(&mut self) {
        let mut front = Vec3::ZERO;
        front.x = self.orientation.x.to_radians().cos() * self.orientation.y.to_radians().sin();
        front.y = self.orientation.x.to_radians().cos() * self.orientation.y.to_radians().cos();
        front.z = -self.orientation.x.to_radians().sin();

        self.forward = front.normalize();
        self.right = self.forward.cross(Vec3::Z).normalize();
        self.up = self.right.cross(self.forward).normalize();
    }
}

impl Default for FpsCamera {
    fn default() -> Self {
        Self {
            rotation: Quat::IDENTITY,
            forward: Vec3::Y,
            right: -Vec3::X,
            up: Vec3::Z,
            position: Vec3::ZERO,
            target_position: Vec3::ZERO,
            orientation: Vec2::ZERO,
            speed_mul: 1.0,
        }
    }
}

impl CameraController for FpsCamera {
    fn update(
        &mut self,
        tween: &mut Option<Tween>,
        input: &crate::input::InputState,
        delta_time: f32,
        smooth_movement: bool,
    ) {
        let mut speed = delta_time * 25.0;
        let mut absolute = false;
        if input.shift() {
            speed *= 3.0;
        }
        if input.ctrl() {
            speed *= 0.10;
        }
        // We're gonna have to go right to... LUDICROUS SPEED
        if input.is_key_down(Key::Space) {
            speed *= 10.0;
        }

        if input.is_key_down(Key::AltLeft) {
            absolute = true;
        }

        let mut direction = Vec3::ZERO;

        if absolute {
            if input.is_key_down(Key::KeyW) {
                direction += self.forward.flatten_xy(Vec3::X);
            }
            if input.is_key_down(Key::KeyS) {
                direction -= self.forward.flatten_xy(Vec3::X);
            }

            if input.is_key_down(Key::KeyA) {
                direction -= self.right.flatten_xy(Vec3::Y);
            }
            if input.is_key_down(Key::KeyD) {
                direction += self.right.flatten_xy(Vec3::Y);
            }

            if input.is_key_down(Key::KeyQ) {
                direction -= Vec3::Z;
            }
            if input.is_key_down(Key::KeyE) {
                direction += Vec3::Z;
            }
        } else {
            if input.is_key_down(Key::KeyW) {
                direction += self.forward;
            }
            if input.is_key_down(Key::KeyS) {
                direction -= self.forward;
            }

            if input.is_key_down(Key::KeyA) {
                direction -= self.right;
            }
            if input.is_key_down(Key::KeyD) {
                direction += self.right;
            }

            if input.is_key_down(Key::KeyQ) {
                direction -= self.up;
            }
            if input.is_key_down(Key::KeyE) {
                direction += self.up;
            }
        }

        speed *= self.speed_mul;

        // Cancel tween if the user moves the camera
        if tween.is_some() && direction.length() > 0.0 {
            *tween = None;
        }

        if let Some(tween) = tween {
            self.target_position = tween.update_pos().unwrap_or(self.target_position);
            self.orientation = tween.update_angle().unwrap_or(self.orientation);
        } else {
            self.target_position += direction * speed;
        }

        if tween.as_ref().is_some_and(Tween::is_finished) {
            *tween = None;
        }

        if smooth_movement {
            self.position = self
                .position
                .lerp(self.target_position, (delta_time * 20.0).min(1.0));
        } else {
            self.position = self.target_position;
        }

        self.orientation.x = self.orientation.x.clamp(-89.9, 89.9);
        self.orientation.y %= 360.0;

        self.update_vectors();

        self.rotation =
            Quat::from_rotation_z(-self.orientation.y.to_radians() + std::f32::consts::FRAC_PI_2)
                * Quat::from_rotation_y(self.orientation.x.to_radians());
    }

    fn update_mouse(&mut self, tween: &mut Option<Tween>, delta: Vec2, scroll_y: f32) {
        self.orientation += Vec2::new(delta.y * 0.8, delta.x) * 0.15;
        self.speed_mul = (self.speed_mul + scroll_y * 0.005).clamp(0.0, 5.0);

        // Cancel angle tween if the user rotates the camera
        if tween.as_ref().map_or(false, |t| t.angle_movement.is_some()) {
            *tween = None;
        }

        self.update_vectors();
    }

    fn position_target(&self) -> Vec3 {
        self.position
    }

    fn position(&self) -> Vec3 {
        self.position
    }

    fn rotation(&self) -> Quat {
        self.rotation
    }

    fn forward(&self) -> Vec3 {
        self.forward
    }

    fn right(&self) -> Vec3 {
        self.right
    }

    fn up(&self) -> Vec3 {
        self.up
    }

    fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.forward, Vec3::Z)
    }

    fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.target_position = position;
    }
}

// use alkahest_data::occlusion::AABB;
// use {Mat4, Quat, Vec2, Vec3};
// use hecs::Entity;
// use winit::event::Key;

// use crate::{
//     input::InputState,
//     render::tween::{self, Tween},
// };

// #[derive(Clone)]
// pub struct FpsCamera {
//     pub orientation: Vec2,
//     pub rotation: Quat,

//     pub front: Vec3,
//     pub right: Vec3,
//     pub up: Vec3,
//     pub position: Vec3,
//     pub speed_mul: f32,
//     pub fov: f32,
//     pub near: f32,

//     pub view_matrix: Mat4,
//     pub projection_matrix: Mat4,
//     pub projection_view_matrix: Mat4,
//     pub projection_view_matrix_inv: Mat4,

//     pub tween: Option<Tween>,
//     pub driving: Option<Entity>,
// }

// impl Default for FpsCamera {
//     fn default() -> Self {
//         Self {
//             rotation: Quat::IDENTITY,
//             front: Vec3::Y,
//             right: -Vec3::X,
//             up: Vec3::Z,
//             position: Vec3::ZERO,
//             orientation: Vec2::ZERO,
//             speed_mul: 1.0,
//             fov: 90.0,
//             near: 0.0001,
//             view_matrix: Mat4::IDENTITY,
//             projection_matrix: Mat4::IDENTITY,
//             projection_view_matrix: Mat4::IDENTITY,
//             projection_view_matrix_inv: Mat4::IDENTITY,
//             tween: None,
//             driving: None,
//         }
//     }
// }

// fn mut dir.flatten_xy(Vec3, default: Vec3) -> Vec3 {
//     dir[2] = 0.0;
//     dir.try_normalize().unwrap_or(default)
// }

// impl FpsCamera {
//     fn update_vectors(&mut self) {
//         let mut front = Vec3::ZERO;
//         front.x = self.orientation.x.to_radians().cos() * self.orientation.y.to_radians().sin();
//         front.y = self.orientation.x.to_radians().cos() * self.orientation.y.to_radians().cos();
//         front.z = -self.orientation.x.to_radians().sin();

//         self.front = front.normalize();
//         self.right = self.front.cross(Vec3::Z).normalize();
//         self.up = self.right.cross(self.front).normalize();
//     }

//     pub fn update_mouse(&mut self, mouse_delta: Vec2) {
//         self.orientation += Vec2::new(mouse_delta.y * 0.8, mouse_delta.x) * 0.15;
//         // Cancel angle tween if the user rotates the camera
//         if self
//             .tween
//             .as_ref()
//             .map_or(false, |t| t.angle_movement.is_some())
//         {
//             self.tween = None;
//         }
//         self.update_vectors();
//     }

//     pub fn update(&mut self, input: &InputState, window_size: (u32, u32), delta: f32) {
//         let mut speed = delta * 35.0;
//         let mut absolute = false;
//         if input.shift() {
//             speed *= 3.0;
//         }
//         if input.ctrl() {
//             speed *= 0.10;
//         }
//         // We're gonna have to go right to... LUDICROUS SPEED
//         if input.is_key_down(Key::Space) {
//             speed *= 10.0;
//         }

//         if input.is_key_down(Key::LAlt) {
//             absolute = true;
//         }

//         let mut direction = Vec3::ZERO;

//         if absolute {
//             if input.is_key_down(Key::W) {
//                 direction += self.front.flatten_xy(Vec3::X);
//             }
//             if input.is_key_down(Key::S) {
//                 direction -= self.front.flatten_xy(Vec3::X);
//             }

//             if input.is_key_down(Key::A) {
//                 direction -= self.right.flatten_xy(Vec3::Y);
//             }
//             if input.is_key_down(Key::D) {
//                 direction += self.right.flatten_xy(Vec3::Y);
//             }

//             if input.is_key_down(Key::Q) {
//                 direction -= Vec3::Z;
//             }
//             if input.is_key_down(Key::E) {
//                 direction += Vec3::Z;
//             }
//         } else {
//             if input.is_key_down(Key::W) {
//                 direction += self.front;
//             }
//             if input.is_key_down(Key::S) {
//                 direction -= self.front;
//             }

//             if input.is_key_down(Key::A) {
//                 direction -= self.right;
//             }
//             if input.is_key_down(Key::D) {
//                 direction += self.right;
//             }

//             if input.is_key_down(Key::Q) {
//                 direction -= self.up;
//             }
//             if input.is_key_down(Key::E) {
//                 direction += self.up;
//             }
//         }

//         speed *= self.speed_mul;

//         // Cancel tween if the user moves the camera
//         if self.tween.is_some() && direction.length() > 0.0 {
//             self.tween = None;
//         }

//         if let Some(tween) = &mut self.tween {
//             self.position = tween.update_pos().unwrap_or(self.position);
//             self.orientation = tween.update_angle().unwrap_or(self.orientation);
//         } else {
//             self.position += direction * speed;
//         }

//         if self.tween.as_ref().is_some_and(Tween::is_finished) {
//             self.tween = None;
//         }

//         self.orientation.x = self.orientation.x.clamp(-89.9, 89.9);
//         self.orientation.y %= 360.0;

//         self.update_vectors();

//         self.view_matrix = self.calculate_matrix();
//         self.projection_matrix = Mat4::perspective_infinite_reverse_rh(
//             self.fov.to_radians(),
//             window_size.0 as f32 / window_size.1 as f32,
//             self.near,
//         );
//         self.projection_view_matrix = self.projection_matrix * self.view_matrix;
//         self.projection_view_matrix_inv = self.projection_view_matrix.inverse();

//         self.rotation =
//             Quat::from_rotation_z(-self.orientation.y.to_radians() + std::f32::consts::FRAC_PI_2)
//                 * Quat::from_rotation_y(self.orientation.x.to_radians());
//     }

//     fn calculate_matrix(&self) -> Mat4 {
//         Mat4::look_at_rh(self.position, self.position + self.front, Vec3::Z)
//     }

//     // pub fn rotation(&self) -> Quat {
//     //     Quat::from_rotation_y(self.orientation.y.to_radians())
//     //         * Quat::from_rotation_x(self.orientation.x.to_radians())
//     // }

//     /// corners[0] - near bottom left
//     /// corners[1] - near bottom right
//     /// corners[2] - near top left
//     /// corners[3] - near top right
//     /// corners[4] - far bottom left
//     /// corners[5] - far bottom right
//     /// corners[6] - far top left
//     /// corners[7] - far top right
//     pub fn calculate_frustum_corners(inv_matrix: &Mat4) -> [Vec3; 8] {
//         let mut corners = [Vec3::ZERO; 8];

//         const NDC_CORNERS: [Vec3; 8] = [
//             Vec3::new(-1.0, -1.0, 0.0),
//             Vec3::new(-1.0, -1.0, 1.0),
//             Vec3::new(-1.0, 1.0, 0.0),
//             Vec3::new(-1.0, 1.0, 1.0),
//             Vec3::new(1.0, -1.0, 0.0),
//             Vec3::new(1.0, -1.0, 1.0),
//             Vec3::new(1.0, 1.0, 0.0),
//             Vec3::new(1.0, 1.0, 1.0),
//         ];

//         for (i, p) in NDC_CORNERS.iter().enumerate() {
//             corners[i] = inv_matrix.project_point3(*p);
//         }

//         corners
//     }

//     pub fn build_cascade(
//         &self,
//         light_dir: Vec3,
//         view: Mat4,
//         cascade_z_start: f32,
//         cascade_z_end: f32,
//         aspect_ratio: f32,
//     ) -> Mat4 {
//         let proj = Mat4::perspective_rh(
//             self.fov.to_radians(),
//             aspect_ratio,
//             cascade_z_start,
//             cascade_z_end,
//         );

//         let frustum_corners = Self::calculate_frustum_corners(&(proj * view).inverse());
//         let frustum_center = frustum_corners.iter().sum::<Vec3>() / frustum_corners.len() as f32;

//         // let light_view = Mat4::look_at_rh(Vec3::ZERO, light_dir, Vec3::Z);
//         let light_view = Mat4::look_at_rh(frustum_center + light_dir, frustum_center, Vec3::Z);

//         // Initialize min and max values
//         let mut min_x = f32::MAX;
//         let mut max_x = f32::MIN;
//         let mut min_y = f32::MAX;
//         let mut max_y = f32::MIN;
//         let mut min_z = f32::MAX;
//         let mut max_z = f32::MIN;

//         // Calculate min and max values for the transformed corners
//         for v in &frustum_corners {
//             let trf = light_view.transform_point3(*v);
//             min_x = min_x.min(trf.x);
//             max_x = max_x.max(trf.x);
//             min_y = min_y.min(trf.y);
//             max_y = max_y.max(trf.y);
//             min_z = min_z.min(trf.z);
//             max_z = max_z.max(trf.z);
//         }

//         // Tune this according to the scene
//         const Z_MULT: f32 = 7.5;
//         let min_z = if min_z < 0.0 {
//             min_z * Z_MULT
//         } else {
//             min_z / Z_MULT
//         };

//         let max_z = if max_z < 0.0 {
//             max_z / Z_MULT
//         } else {
//             max_z * Z_MULT
//         };

//         let light_projection = Mat4::orthographic_rh(min_x, max_x, min_y, max_y, min_z, max_z);

//         light_projection * light_view
//     }

//     pub fn is_point_visible(&self, point: Vec3) -> bool {
//         let point_transformed = self.projection_view_matrix.project_point3(point);

//         point_transformed.z >= 0.0
//     }

//     pub fn is_aabb_visible(&self, bb: &AABB) -> bool {
//         let mut corners = [Vec3::ZERO; 8];

//         let mut i = 0;
//         for x in [bb.min.x, bb.max.x].iter() {
//             for y in [bb.min.y, bb.max.y].iter() {
//                 for z in [bb.min.z, bb.max.z].iter() {
//                     corners[i] = Vec3::new(*x, *y, *z);
//                     i += 1;
//                 }
//             }
//         }

//         for corner in corners.iter() {
//             if self.is_point_visible(*corner) {
//                 return true;
//             }
//         }

//         false
//     }

//     pub fn focus(&mut self, pos: Vec3, distance: f32) {
//         self.tween = Some(Tween::new(
//             tween::ease_out_exponential,
//             Some((self.position, pos - self.front * distance)),
//             None,
//             0.70,
//         ));
//     }

//     pub fn focus_aabb(&mut self, bb: &AABB) {
//         let center = bb.center();
//         let radius = bb.radius();

//         self.focus(center, radius);
//     }

//     // Calculate angle to point camera at pos.
//     // The angle has a minimal diff to current camera angle.
//     pub fn get_look_angle(&self, pos: Vec3) -> Vec2 {
//         let dir = pos - self.position;
//         let inv_r = dir.length_recip();
//         if inv_r.is_infinite() {
//             self.orientation
//         } else {
//             let theta = dir.x.atan2(dir.y).to_degrees();
//             let mut diff = (theta - self.orientation.y).rem_euclid(360.0);
//             if diff > 180.0 {
//                 diff -= 360.0;
//             }
//             Vec2::new(
//                 (dir.z * inv_r).acos().to_degrees() - 90.0,
//                 self.orientation.y + diff,
//             )
//         }
//     }
// }
