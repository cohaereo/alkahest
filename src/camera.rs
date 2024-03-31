use alkahest_data::occlusion::AABB;
use glam::{Mat4, Quat, Vec2, Vec3};
use hecs::Entity;
use winit::event::VirtualKeyCode;

use crate::{
    input::InputState,
    render::tween::{self, Tween},
};

#[derive(Clone)]
pub struct FpsCamera {
    pub orientation: Vec2,
    pub rotation: Quat,

    pub front: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub position: Vec3,
    pub speed_mul: f32,
    pub fov: f32,
    pub near: f32,

    pub view_matrix: Mat4,
    pub projection_matrix: Mat4,
    pub projection_view_matrix: Mat4,
    pub projection_view_matrix_inv: Mat4,

    pub tween: Option<Tween>,
    pub driving: Option<Entity>,
}

impl Default for FpsCamera {
    fn default() -> Self {
        Self {
            rotation: Quat::IDENTITY,
            front: Vec3::Y,
            right: -Vec3::X,
            up: Vec3::Z,
            position: Vec3::ZERO,
            orientation: Vec2::ZERO,
            speed_mul: 1.0,
            fov: 90.0,
            near: 0.0001,
            view_matrix: Mat4::IDENTITY,
            projection_matrix: Mat4::IDENTITY,
            projection_view_matrix: Mat4::IDENTITY,
            projection_view_matrix_inv: Mat4::IDENTITY,
            tween: None,
            driving: None,
        }
    }
}

fn flatten_xy(mut dir: Vec3, default: Vec3) -> Vec3 {
    dir[2] = 0.0;
    dir.try_normalize().unwrap_or(default)
}

impl FpsCamera {
    fn update_vectors(&mut self) {
        let mut front = Vec3::ZERO;
        front.x = self.orientation.x.to_radians().cos() * self.orientation.y.to_radians().sin();
        front.y = self.orientation.x.to_radians().cos() * self.orientation.y.to_radians().cos();
        front.z = -self.orientation.x.to_radians().sin();

        self.front = front.normalize();
        self.right = self.front.cross(Vec3::Z).normalize();
        self.up = self.right.cross(self.front).normalize();
    }

    pub fn update_mouse(&mut self, mouse_delta: Vec2) {
        self.orientation += Vec2::new(mouse_delta.y * 0.8, mouse_delta.x) * 0.15;
        // Cancel angle tween if the user rotates the camera
        if self
            .tween
            .as_ref()
            .map_or(false, |t| t.angle_movement.is_some())
        {
            self.tween = None;
        }
        self.update_vectors();
    }

    pub fn update(&mut self, input: &InputState, window_size: (u32, u32), delta: f32) {
        let mut speed = delta * 35.0;
        let mut absolute = false;
        if input.shift() {
            speed *= 3.0;
        }
        if input.ctrl() {
            speed *= 0.10;
        }
        // We're gonna have to go right to... LUDICROUS SPEED
        if input.is_key_down(VirtualKeyCode::Space) {
            speed *= 10.0;
        }

        if input.is_key_down(VirtualKeyCode::LAlt) {
            absolute = true;
        }

        let mut direction = Vec3::ZERO;

        if absolute {
            if input.is_key_down(VirtualKeyCode::W) {
                direction += flatten_xy(self.front, Vec3::X);
            }
            if input.is_key_down(VirtualKeyCode::S) {
                direction -= flatten_xy(self.front, Vec3::X);
            }

            if input.is_key_down(VirtualKeyCode::A) {
                direction -= flatten_xy(self.right, Vec3::Y);
            }
            if input.is_key_down(VirtualKeyCode::D) {
                direction += flatten_xy(self.right, Vec3::Y);
            }

            if input.is_key_down(VirtualKeyCode::Q) {
                direction -= Vec3::Z;
            }
            if input.is_key_down(VirtualKeyCode::E) {
                direction += Vec3::Z;
            }
        } else {
            if input.is_key_down(VirtualKeyCode::W) {
                direction += self.front;
            }
            if input.is_key_down(VirtualKeyCode::S) {
                direction -= self.front;
            }

            if input.is_key_down(VirtualKeyCode::A) {
                direction -= self.right;
            }
            if input.is_key_down(VirtualKeyCode::D) {
                direction += self.right;
            }

            if input.is_key_down(VirtualKeyCode::Q) {
                direction -= self.up;
            }
            if input.is_key_down(VirtualKeyCode::E) {
                direction += self.up;
            }
        }

        speed *= self.speed_mul;

        // Cancel tween if the user moves the camera
        if direction.length() > 0.0 {
            if let Some(t) = self.tween.as_mut() {
                t.abort()
            }
        }

        if let Some(tween) = &mut self.tween {
            if tween.is_aborted() {
                self.position += direction * speed;
            } else {
                self.position = tween.update_pos().unwrap_or(self.position);
                self.orientation = tween.update_angle().unwrap_or(self.orientation);
            }
        } else {
            self.position += direction * speed;
        }

        if self.tween.as_ref().is_some_and(Tween::is_finished) {
            self.tween = None;
        }

        self.orientation.x = self.orientation.x.clamp(-89.9, 89.9);
        self.orientation.y %= 360.0;

        self.update_vectors();

        self.view_matrix = self.calculate_matrix();
        self.projection_matrix = Mat4::perspective_infinite_reverse_rh(
            self.fov.to_radians(),
            window_size.0 as f32 / window_size.1 as f32,
            self.near,
        );
        self.projection_view_matrix = self.projection_matrix * self.view_matrix;
        self.projection_view_matrix_inv = self.projection_view_matrix.inverse();

        self.rotation =
            Quat::from_rotation_z(-self.orientation.y.to_radians() + std::f32::consts::FRAC_PI_2)
                * Quat::from_rotation_y(self.orientation.x.to_radians());
    }

    fn calculate_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.front, Vec3::Z)
    }

    // pub fn rotation(&self) -> Quat {
    //     Quat::from_rotation_y(self.orientation.y.to_radians())
    //         * Quat::from_rotation_x(self.orientation.x.to_radians())
    // }

    /// corners[0] - near bottom left
    /// corners[1] - near bottom right
    /// corners[2] - near top left
    /// corners[3] - near top right
    /// corners[4] - far bottom left
    /// corners[5] - far bottom right
    /// corners[6] - far top left
    /// corners[7] - far top right
    pub fn calculate_frustum_corners(inv_matrix: &Mat4) -> [Vec3; 8] {
        let mut corners = [Vec3::ZERO; 8];

        const NDC_CORNERS: [Vec3; 8] = [
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, 0.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
        ];

        for (i, p) in NDC_CORNERS.iter().enumerate() {
            corners[i] = inv_matrix.project_point3(*p);
        }

        corners
    }

    pub fn build_cascade(
        &self,
        light_dir: Vec3,
        view: Mat4,
        cascade_z_start: f32,
        cascade_z_end: f32,
        aspect_ratio: f32,
    ) -> Mat4 {
        let proj = Mat4::perspective_rh(
            self.fov.to_radians(),
            aspect_ratio,
            cascade_z_start,
            cascade_z_end,
        );

        let frustum_corners = Self::calculate_frustum_corners(&(proj * view).inverse());
        let frustum_center = frustum_corners.iter().sum::<Vec3>() / frustum_corners.len() as f32;

        // let light_view = Mat4::look_at_rh(Vec3::ZERO, light_dir, Vec3::Z);
        let light_view = Mat4::look_at_rh(frustum_center + light_dir, frustum_center, Vec3::Z);

        // Initialize min and max values
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;

        // Calculate min and max values for the transformed corners
        for v in &frustum_corners {
            let trf = light_view.transform_point3(*v);
            min_x = min_x.min(trf.x);
            max_x = max_x.max(trf.x);
            min_y = min_y.min(trf.y);
            max_y = max_y.max(trf.y);
            min_z = min_z.min(trf.z);
            max_z = max_z.max(trf.z);
        }

        // Tune this according to the scene
        const Z_MULT: f32 = 7.5;
        let min_z = if min_z < 0.0 {
            min_z * Z_MULT
        } else {
            min_z / Z_MULT
        };

        let max_z = if max_z < 0.0 {
            max_z / Z_MULT
        } else {
            max_z * Z_MULT
        };

        let light_projection = Mat4::orthographic_rh(min_x, max_x, min_y, max_y, min_z, max_z);

        light_projection * light_view
    }

    pub fn is_point_visible(&self, point: Vec3) -> bool {
        let point_transformed = self.projection_view_matrix.project_point3(point);

        point_transformed.z >= 0.0
    }

    pub fn is_aabb_visible(&self, bb: &AABB) -> bool {
        let mut corners = [Vec3::ZERO; 8];

        let mut i = 0;
        for x in [bb.min.x, bb.max.x].iter() {
            for y in [bb.min.y, bb.max.y].iter() {
                for z in [bb.min.z, bb.max.z].iter() {
                    corners[i] = Vec3::new(*x, *y, *z);
                    i += 1;
                }
            }
        }

        for corner in corners.iter() {
            if self.is_point_visible(*corner) {
                return true;
            }
        }

        false
    }

    pub fn focus(&mut self, pos: Vec3, distance: f32) {
        self.tween = Some(Tween::new(
            tween::ease_out_exponential,
            Some((self.position, pos - self.front * distance)),
            None,
            0.70,
        ));
    }

    pub fn focus_aabb(&mut self, bb: &AABB) {
        let center = bb.center();
        let radius = bb.radius();

        self.focus(center, radius);
    }

    // Calculate angle to point camera at pos.
    // The angle has a minimal diff to current camera angle.
    pub fn get_look_angle(&self, pos: Vec3) -> Vec2 {
        get_look_angle(self.orientation, self.position, pos)
    }
}

pub fn get_look_angle(start_angle: Vec2, pos1: Vec3, pos2: Vec3) -> Vec2 {
    let dir = pos2 - pos1;
    let inv_r = dir.length_recip();
    if inv_r.is_infinite() {
        start_angle
    } else {
        let theta = dir.x.atan2(dir.y).to_degrees();
        let mut diff = (theta - start_angle.y).rem_euclid(360.0);
        if diff > 180.0 {
            diff -= 360.0;
        }
        Vec2::new(
            (dir.z * inv_r).acos().to_degrees() - 90.0,
            start_angle.y + diff,
        )
    }
}
/* TODO: This seems to be acurate, but it's giving funky results for turning speeds
pub fn get_look_angle_difference(start_angle: Vec2, pos1: Vec3, pos2: Vec3) -> f32 {
    let dir = pos2 - pos1;
    let theta = start_angle.y.to_radians();
    let phi =  start_angle.x.to_radians();
    let old_dir = Vec3::new(
        phi.cos() * theta.cos(),
        phi.cos() * theta.sin(),
        -phi.sin(),
    );
    dir.angle_between(old_dir).to_degrees()
}
*/
