use glam::Vec3;
use std::time::Instant;

#[derive(Clone)]
pub struct Tween {
    func: fn(f32) -> f32,
    pub start: Vec3,
    pub end: Vec3,
    pub last_pos: Vec3,
    pub start_time: Instant,
    pub duration: f32,
}

impl Tween {
    pub fn new(func: fn(f32) -> f32, start: Vec3, end: Vec3, duration: f32) -> Self {
        Self {
            func,
            start,
            end,
            last_pos: start,
            start_time: Instant::now(),
            duration,
        }
    }

    pub fn update(&mut self) -> Vec3 {
        let time = self.start_time.elapsed().as_secs_f32();
        let t = (time / self.duration).clamp(0., 1.);
        let s = (self.func)(t);

        let new_pos = self.start.lerp(self.end, s);
        self.last_pos = new_pos;
        new_pos
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
