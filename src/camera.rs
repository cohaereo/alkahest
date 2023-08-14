use glam::{Mat4, Vec2, Vec3};
use winit::event::VirtualKeyCode;

use crate::input::InputState;

#[derive(Clone)]
pub struct FpsCamera {
    orientation: Vec2,
    pub front: Vec3,
    pub right: Vec3,
    pub position: Vec3,
    pub speed_mul: f32,
}

impl Default for FpsCamera {
    fn default() -> Self {
        Self {
            front: Vec3::Y,
            right: -Vec3::X,
            position: Vec3::ZERO,
            orientation: Vec2::ZERO,
            speed_mul: 1.0,
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
            direction -= Vec3::Y;
        }
        if input.is_key_down(VirtualKeyCode::E) {
            direction += Vec3::Y;
        }

        self.position += direction * speed;

        self.orientation.x = self.orientation.x.clamp(-89.9, 89.9);

        self.update_vectors();
    }

    pub fn calculate_matrix(&mut self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.front, Vec3::Z)
    }

    // pub fn rotation(&self) -> Quat {
    //     Quat::from_rotation_y(self.orientation.y.to_radians())
    //         * Quat::from_rotation_x(self.orientation.x.to_radians())
    // }
}
