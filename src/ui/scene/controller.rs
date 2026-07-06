use std::time::Instant;

use alkahest_render::{Renderer, camera::Camera};
use egui::{Response, Ui};
use glam::{Quat, Vec2, Vec3};

pub enum CameraController {
    Orbit {
        target: Vec3,
        distance: f32,
        yaw_pitch: Vec2,
    },
    FirstPerson {
        ln_speed: f32,
        yaw_pitch: Vec2,
    },
}

impl CameraController {
    pub const DEFAULT_YAW_PITCH: Vec2 = Vec2::new(220.0, 25.0);
    pub fn new_orbit(target: Vec3, distance: f32) -> Self {
        Self::Orbit {
            target,
            distance,
            yaw_pitch: Self::DEFAULT_YAW_PITCH,
        }
    }

    pub fn new_first_person() -> Self {
        Self::FirstPerson {
            ln_speed: 3.2,
            yaw_pitch: Vec2::ZERO,
        }
    }

    pub fn update(&mut self, camera: &mut Camera, ui: &Ui, response: &Response, delta_time: f32) {
        match self {
            Self::Orbit {
                target,
                distance,
                yaw_pitch,
            } => {
                if camera.draw_grid {
                    const GRID_CELL_SIZE: f32 = 1.0;
                    const GRID_DIM: usize = 10;
                    const GRID_SIZE: f32 = GRID_CELL_SIZE * GRID_DIM as f32;
                    const GRID_HALF_SIZE: f32 = GRID_SIZE / 2.0;

                    for i in 0..GRID_DIM + 1 {
                        let pos_x = Vec3::new(
                            i as f32 * GRID_CELL_SIZE - GRID_HALF_SIZE,
                            -GRID_HALF_SIZE,
                            0.0,
                        );
                        let pos_y = Vec3::new(
                            -GRID_HALF_SIZE,
                            i as f32 * GRID_CELL_SIZE - GRID_HALF_SIZE,
                            0.0,
                        );

                        Renderer::instance().immediate.lock().line(
                            pos_x,
                            pos_x + Vec3::new(0.0, GRID_SIZE, 0.0),
                            0x666666,
                        );
                        Renderer::instance().immediate.lock().line(
                            pos_y,
                            pos_y + Vec3::new(GRID_SIZE, 0.0, 0.0),
                            0x666666,
                        );
                    }
                }

                if response.hovered() {
                    let scroll_delta = ui.input(|i| i.raw_scroll_delta);
                    *distance += -scroll_delta.y / 250.0;
                    *distance = distance.clamp(0.01, 1000.0);
                }
                let real_distance = 2.0f32.powf(*distance * 0.3) - 0.9;

                let drag_delta = response.drag_delta();
                // Rotate
                if response.dragged_by(egui::PointerButton::Primary) {
                    let drag_delta_scaled = (drag_delta / 5.0) * egui::vec2(-1.0, 1.3);
                    *yaw_pitch += Vec2::new(drag_delta_scaled.x, drag_delta_scaled.y);
                    yaw_pitch.y = yaw_pitch.y.clamp(-89.0, 89.0);
                }

                if response.dragged_by(egui::PointerButton::Secondary) {
                    let delta_adjusted = (drag_delta / 350.0) * real_distance;
                    *target -= camera.right() * delta_adjusted.x;
                    *target += camera.up() * delta_adjusted.y;
                }

                if response.dragged() {
                    Renderer::instance()
                        .immediate
                        .lock()
                        .cross(*target, 0.15, 0xffffff);
                }

                camera.position = *target - camera.forward() * real_distance;
            }
            Self::FirstPerson {
                ln_speed,
                yaw_pitch,
            } => {
                if response.hovered() {
                    let scroll_delta = ui.input(|i| i.raw_scroll_delta);
                    if scroll_delta.y != 0.0 {
                        ui.memory_mut(|m| {
                            m.data
                                .insert_temp("scene_last_speed_change".into(), Instant::now());
                        });
                    }
                    *ln_speed += scroll_delta.y / 250.0;
                    *ln_speed = ln_speed.clamp(-10.0, 5.0);
                }

                let mut movement = Vec3::ZERO;
                ui.input(|i| {
                    if i.key_down(egui::Key::W) {
                        movement += camera.forward();
                    }
                    if i.key_down(egui::Key::S) {
                        movement -= camera.forward();
                    }
                    if i.key_down(egui::Key::A) {
                        movement -= camera.right();
                    }
                    if i.key_down(egui::Key::D) {
                        movement += camera.right();
                    }
                    if i.key_down(egui::Key::Q) {
                        movement -= camera.up();
                    }
                    if i.key_down(egui::Key::E) {
                        movement += camera.up();
                    }
                    movement = movement.normalize_or(Vec3::ZERO);
                    if i.modifiers.ctrl {
                        movement /= 5.0;
                    }
                    if i.modifiers.shift {
                        movement *= 5.0;
                    }
                    if i.key_down(egui::Key::Space) {
                        movement *= 5.0;
                    }
                });

                if response.dragged_by(egui::PointerButton::Primary)
                    || response.dragged_by(egui::PointerButton::Secondary)
                {
                    let drag_delta = response.drag_delta();
                    let drag_delta_scaled = (drag_delta / 10.0) * egui::vec2(-1.0, 1.3);
                    *yaw_pitch += Vec2::new(drag_delta_scaled.x, drag_delta_scaled.y);
                    yaw_pitch.y = yaw_pitch.y.clamp(-89.0, 89.0);
                }
                camera.position += movement * delta_time * ln_speed.exp();
            }
        }

        self.update_rotation(camera);
    }

    pub fn set_yaw_pitch(&mut self, yaw_pitch: Vec2) {
        match self {
            CameraController::Orbit { yaw_pitch: yp, .. } => {
                *yp = yaw_pitch;
            }
            CameraController::FirstPerson { yaw_pitch: yp, .. } => {
                *yp = yaw_pitch;
            }
        }
    }

    pub fn yaw_pitch(&self) -> Vec2 {
        match self {
            CameraController::Orbit { yaw_pitch, .. } => *yaw_pitch,
            CameraController::FirstPerson { yaw_pitch, .. } => *yaw_pitch,
        }
    }

    pub fn update_rotation(&mut self, camera: &mut Camera) {
        match self {
            CameraController::Orbit {
                target,
                yaw_pitch,
                distance,
            } => {
                camera.rotation = Quat::from_rotation_z(yaw_pitch.x.to_radians())
                    * Quat::from_rotation_y(yaw_pitch.y.to_radians());
                let real_distance = 2.0f32.powf(*distance * 0.3) - 0.9;
                camera.position = *target - camera.forward() * real_distance;
            }
            CameraController::FirstPerson { yaw_pitch, .. } => {
                camera.rotation = Quat::from_rotation_z(yaw_pitch.x.to_radians())
                    * Quat::from_rotation_y(yaw_pitch.y.to_radians());
            }
        }
    }

    pub fn speed(&self) -> f32 {
        match self {
            Self::Orbit { .. } => 1.0,
            Self::FirstPerson { ln_speed, .. } => ln_speed.exp(),
        }
    }
}

pub fn egui_to_glam_vec2(vec: egui::Vec2) -> glam::Vec2 {
    glam::Vec2::new(vec.x, vec.y)
}
