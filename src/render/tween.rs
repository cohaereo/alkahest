use std::time::Instant;

use glam::{Vec2, Vec3};

#[derive(Clone)]
pub struct Tween {
    func: fn(f32) -> f32,
    pub pos_movement: Option<(Vec3, Vec3)>,
    pub last_pos: Option<Vec3>,
    pub angle_movement: Option<(Vec2, Vec2)>,
    pub last_angle: Option<Vec2>,
    pub start_time: Instant,
    pub duration: f32,
}

impl Tween {
    pub fn new(
        func: fn(f32) -> f32,
        pos_movement: Option<(Vec3, Vec3)>,
        angle_movement: Option<(Vec2, Vec2)>,
        duration: f32,
    ) -> Self {
        Self {
            func,
            pos_movement,
            last_pos: pos_movement.map(|pos| pos.0),
            angle_movement,
            last_angle: angle_movement.map(|angle| angle.0),
            start_time: Instant::now(),
            duration,
        }
    }

    pub fn update_pos(&mut self) -> Option<Vec3> {
        let time = self.start_time.elapsed().as_secs_f32();
        let t = (time / self.duration).clamp(0., 1.);
        let s = (self.func)(t);

        let new_pos = self.pos_movement.map(|pos| pos.0.lerp(pos.1, s));
        self.last_pos = new_pos;
        new_pos
    }

    pub fn update_angle(&mut self) -> Option<Vec2> {
        let time = self.start_time.elapsed().as_secs_f32();
        let t = (time / self.duration).clamp(0., 1.);
        let s = (self.func)(t);

        let new_angle = self.angle_movement.map(|angle| angle.0.lerp(angle.1, s));
        self.last_angle = new_angle;
        new_angle
    }

    pub fn is_finished(&self) -> bool {
        self.start_time.elapsed().as_secs_f32() >= self.duration
    }
}

// https://easings.net/#easeOutExpo
pub fn ease_out_exponential(x: f32) -> f32 {
    if x == 1.0 {
        1.0
    } else {
        1.0 - 2f32.powf(-10. * x)
    }
}
