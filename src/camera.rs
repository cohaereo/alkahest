use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use winit::event::VirtualKeyCode;

use crate::input::InputState;

#[derive(Clone)]
pub struct FpsCamera {
    orientation: Vec2,
    pub rotation: Quat,

    pub front: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub position: Vec3,
    pub speed_mul: f32,
    pub fov: f32,

    pub projection_matrix: Mat4,
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
            projection_matrix: Mat4::IDENTITY,
        }
    }
}

impl FpsCamera {
    pub fn update_vectors(&mut self) {
        let mut front = Vec3::ZERO;
        front.x = self.orientation.x.to_radians().cos() * self.orientation.y.to_radians().sin();
        front.y = self.orientation.x.to_radians().cos() * self.orientation.y.to_radians().cos();
        front.z = -self.orientation.x.to_radians().sin();

        self.front = front.normalize();
        self.right = -self.front.cross(Vec3::Z).normalize();
        self.up = self.right.cross(self.front).normalize();

        self.rotation = Mat4::look_at_rh(self.position, self.position + self.front, Vec3::Z)
            .to_scale_rotation_translation()
            .1;
    }

    pub fn update_mouse(&mut self, mouse_delta: Vec2) {
        self.orientation += Vec2::new(mouse_delta.y * 0.8, mouse_delta.x) * 0.15;
        self.update_vectors();
    }

    pub fn update(&mut self, input: &InputState, delta: f32) {
        let mut speed = delta * 35.0;
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

        let mut direction = Vec3::ZERO;
        if input.is_key_down(VirtualKeyCode::W) {
            direction += self.front;
        }
        if input.is_key_down(VirtualKeyCode::S) {
            direction -= self.front;
        }

        if input.is_key_down(VirtualKeyCode::D) {
            direction -= self.right;
        }
        if input.is_key_down(VirtualKeyCode::A) {
            direction += self.right;
        }

        if input.is_key_down(VirtualKeyCode::Q) {
            direction -= Vec3::Z;
        }
        if input.is_key_down(VirtualKeyCode::E) {
            direction += Vec3::Z;
        }

        speed *= self.speed_mul;

        self.position += direction * speed;

        self.orientation.x = self.orientation.x.clamp(-89.9, 89.9);

        self.update_vectors();
    }

    pub fn calculate_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.front, Vec3::Z)
    }

    // pub fn rotation(&self) -> Quat {
    //     Quat::from_rotation_y(self.orientation.y.to_radians())
    //         * Quat::from_rotation_x(self.orientation.x.to_radians())
    // }

    // 0 - near bottom left
    // 1 - near bottom right
    // 2 - near top left
    // 3 - near top right
    // 4 - far bottom left
    // 5 - far bottom right
    // 6 - far top left
    // 7 - far top right
    pub fn calculate_frustum_corners(inv_matrix: Mat4) -> [Vec3; 8] {
        let mut corners = [Vec3::ZERO; 8];

        const NDC_CORNERS: [Vec4; 8] = [
            Vec4::new(-1.0, -1.0, 0.0, 1.0),
            Vec4::new(-1.0, -1.0, 1.0, 1.0),
            Vec4::new(-1.0, 1.0, 0.0, 1.0),
            Vec4::new(-1.0, 1.0, 1.0, 1.0),
            Vec4::new(1.0, -1.0, 0.0, 1.0),
            Vec4::new(1.0, -1.0, 1.0, 1.0),
            Vec4::new(1.0, 1.0, 0.0, 1.0),
            Vec4::new(1.0, 1.0, 1.0, 1.0),
        ];

        for (i, c) in NDC_CORNERS.iter().enumerate() {
            let p = inv_matrix * *c;
            corners[i] = (p / p.w).truncate();
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

        let frustum_corners = Self::calculate_frustum_corners((proj * view).inverse());
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
}
