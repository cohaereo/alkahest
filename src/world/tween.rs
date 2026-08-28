use alkahest_render::camera::Camera;
use glam::{Quat, Vec3};
use hecs::{CommandBuffer, World};

use crate::world::transform::Transform;

#[derive(Clone)]
pub struct Tween {
    func: fn(f32) -> f32,
    pub pos_movement: Option<(Vec3, Vec3)>,
    pub angle_movement: Option<(Quat, Quat)>,
    pub progress: f32,
    pub duration: f32,
}

impl Tween {
    pub fn new(
        func: fn(f32) -> f32,
        pos_movement: Option<(Vec3, Vec3)>,
        angle_movement: Option<(Quat, Quat)>,
        duration: f32,
    ) -> Self {
        Self {
            func,
            pos_movement,
            angle_movement,
            progress: 0.0,
            duration,
        }
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

// https://easings.net/#easeInOutExpo
pub fn _ease_in_out_exponential(x: f32) -> f32 {
    if x == 0.0 {
        0.0
    } else if x == 1.0 {
        1.0
    } else if x < 0.5 {
        2f32.powf(20. * x - 10.) / 2.
    } else {
        (2. - 2f32.powf(-20. * x + 10.)) / 2.
    }
}

fn s_update_tween_maybe_remove(
    translation: &mut Vec3,
    rotation: &mut Quat,
    tween: &mut Tween,
    dt: f32,
) -> bool {
    tween.progress += dt;
    let t = (tween.progress / tween.duration).clamp(0., 1.);
    let s = (tween.func)(t);

    if let Some((start_pos, end_pos)) = tween.pos_movement.as_ref() {
        *translation = start_pos.lerp(*end_pos, s);
    }
    if let Some((start_angle, end_angle)) = tween.angle_movement.as_ref() {
        *rotation = start_angle.lerp(*end_angle, s);
    }
    tween.progress >= tween.duration
}

// TODO: Move Camera into world
pub fn s_update_tweens(world: &mut World, camera: &mut Camera, tween: &mut Option<Tween>, dt: f32) {
    let mut cmd = CommandBuffer::new();
    for (entity, (transform, tween)) in world.query::<(&mut Transform, &mut Tween)>().iter() {
        if s_update_tween_maybe_remove(
            &mut transform.translation,
            &mut transform.rotation,
            tween,
            dt,
        ) {
            cmd.remove_one::<Tween>(entity);
        }
    }
    if let Some(t) = tween
        && s_update_tween_maybe_remove(&mut camera.position, &mut camera.rotation, t, dt)
    {
        *tween = None;
    }
    cmd.run_on(world);
}
