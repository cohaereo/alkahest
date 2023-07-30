use glam::{Mat4, Quat, Vec2, Vec3};

pub struct InputState {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub mouse1: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub space: bool,
}

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
            right: Vec3::Z,
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
        self.orientation += -Vec2::new(mouse_delta.y * 0.8, mouse_delta.x) * 0.15;
        self.update_vectors();
    }

    pub fn update(&mut self, input: &InputState, delta: f32) {
        let mut speed = delta * 35.0;
        if input.shift {
            speed *= 3.0;
        }
        if input.ctrl {
            speed *= 0.25;
        }
        // We're gonna have to go right to... LUDICROUS SPEED
        if input.space {
            speed *= 10.0;
        }

        let mut direction = Vec3::ZERO;
        if input.w {
            direction -= self.front;
        }
        if input.s {
            direction += self.front;
        }

        if input.d {
            direction -= self.right;
        }
        if input.a {
            direction += self.right;
        }
        //
        // if ui.input(|i| i.key_down(egui::Key::Q)) {
        //     direction -= Vec3::Y;
        // }
        // if ui.input(|i| i.key_down(egui::Key::E)) {
        //     direction += Vec3::Y;
        // }

        self.position += direction * speed;

        self.orientation.x = self.orientation.x.clamp(-89.9, 89.9);

        self.update_vectors();
    }

    pub fn calculate_matrix(&mut self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.front, Vec3::Z)
            * Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)) // TODO(cohae): fix this shit
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_rotation_y(self.orientation.y.to_radians())
            * Quat::from_rotation_x(self.orientation.x.to_radians())
    }

    pub fn position(&mut self) -> Vec3 {
        self.position
    }
}

pub fn convert_matrix(m: Mat4) -> Mat4 {
    Mat4::from_cols(
        m.x_axis.truncate().extend(m.w_axis.x),
        m.y_axis.truncate().extend(m.w_axis.y),
        m.z_axis.truncate().extend(m.w_axis.z),
        [0.0, 0.0, 0.0, 1.0].into(),
    )
}
