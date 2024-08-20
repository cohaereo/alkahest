use glam::{Mat4, Quat, Vec2, Vec2Swizzles, Vec3};

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
        }
    }
}

impl CameraController for FpsCamera {
    fn update(
        &mut self,
        tween: &mut Option<Tween>,
        input: &crate::input::InputState,
        delta_time: f32,
        speed_mul: f32,
        smooth_movement: f32,
        // TODO(cohae): Implement smooth look
        _smooth_look: f32,
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

        speed *= speed_mul;

        if input.ctrl() && input.shift() {
            speed = 0.0;
        }

        // Cancel tween if the user moves the camera
        if direction.length() > 0.0 {
            if let Some(t) = tween {
                t.abort();
            }
        }

        if let Some(tween) = tween {
            if tween.is_aborted() {
                self.target_position += direction * speed;
            } else {
                self.target_position = tween.update_pos().unwrap_or(self.target_position);
                self.orientation = tween.update_angle().unwrap_or(self.orientation);
            }
        } else {
            self.target_position += direction * speed;
        }

        if tween.as_ref().is_some_and(Tween::is_finished) {
            *tween = None;
        }

        if smooth_movement > 0.0 {
            self.position = self.position.lerp(
                self.target_position,
                (delta_time * (15.0 / smooth_movement)).min(1.0),
            );
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

    fn update_mouse(&mut self, delta: Vec2, _scroll_y: f32) {
        self.orientation += Vec2::new(delta.y * 0.8, delta.x) * 0.15;

        self.update_vectors();
    }

    fn update_gamepad(&mut self, movement: Vec2, look: Vec2, speed_mul: f32, delta_time: f32) {
        let mut direction = Vec3::ZERO;

        direction += self.forward * movement.y;
        direction += self.right * movement.x;

        let mut speed = delta_time * 25.0;
        speed *= speed_mul;
        self.target_position += direction * speed;

        self.orientation += (look.yx() * Vec2::new(-1., 1.)) * 1.5;

        self.update_vectors();
    }

    fn position_target(&self) -> Vec3 {
        self.position
    }

    fn position(&self) -> Vec3 {
        self.position
    }

    fn orientation(&self) -> Vec2 {
        self.orientation
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

    fn view_angle(&self) -> Vec2 {
        self.orientation
    }

    fn get_look_angle(&self, pos: Vec3) -> Vec2 {
        super::get_look_angle(self.orientation, self.position, pos)
    }

    fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.target_position = position;
    }

    fn set_orientation(&mut self, orientation: Vec2) {
        self.orientation = orientation;
        self.update_vectors();
    }
}
