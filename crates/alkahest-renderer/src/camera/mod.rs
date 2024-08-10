pub mod projection;
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
pub use projection::CameraProjection;

pub mod fps;
pub mod orbit;
pub mod tween;

pub mod viewport;
pub use viewport::Viewport;

use self::{fps::FpsCamera, tween::Tween};
use crate::{
    ecs::culling::Frustum,
    input::InputState,
    tfx::view::{RenderStageSubscriptions, View},
};

pub trait CameraController {
    fn update(
        &mut self,
        tween: &mut Option<Tween>,
        input: &InputState,
        delta_time: f32,
        speed: f32,
        smooth_movement: f32,
        smooth_look: f32,
    );
    fn update_mouse(&mut self, tween: &mut Option<Tween>, delta: Vec2, scroll: f32);

    // TODO(cohae): These might be a bit confusing
    /// Returns the position of the camera
    /// Orbit camera will return the target position instead
    fn position_target(&self) -> Vec3;

    /// Returns the position of the camera "view"
    /// Orbit camera will return the position of the view instead of the target
    fn position(&self) -> Vec3;

    fn orientation(&self) -> Vec2;
    fn rotation(&self) -> Quat;

    fn forward(&self) -> Vec3;
    fn right(&self) -> Vec3;
    fn up(&self) -> Vec3;

    fn view_matrix(&self) -> Mat4;
    fn view_angle(&self) -> Vec2;
    fn get_look_angle(&self, pos: Vec3) -> Vec2;

    fn set_position(&mut self, position: Vec3);
    fn set_orientation(&mut self, orientation: Vec2);
    // fn set_rotation(&mut self, rotation: Quat);
    // fn look_at(&mut self, target: Vec3);
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
pub struct Camera {
    controller: Box<dyn CameraController>,
    viewport: Viewport,

    pub projection: CameraProjection,
    pub tween: Option<Tween>,

    // Aka view matrix
    pub world_to_camera: Mat4,
    pub camera_to_world: Mat4,
    // Aka projection matrix
    pub camera_to_projective: Mat4,
    pub projective_to_camera: Mat4,
    // Aka view+projection matrix
    pub world_to_projective: Mat4,
    pub projective_to_world: Mat4,

    pub frustum: Frustum,

    pub target_pixel_to_projective: Mat4,

    pub speed_mul: f32,
    pub smooth_movement: f32,
    pub smooth_look: f32,
}

impl Camera {
    pub fn new_fps(viewport: Viewport) -> Self {
        Self::new(
            viewport,
            CameraProjection::Perspective {
                fov: 90.0,
                near: 0.01,
            },
            Box::<FpsCamera>::default(),
        )
    }

    pub fn new(
        viewport: Viewport,
        projection: CameraProjection,
        controller: Box<dyn CameraController>,
    ) -> Self {
        let mut camera = Self {
            controller,
            viewport,

            projection,
            tween: None,

            world_to_camera: Mat4::IDENTITY,
            camera_to_world: Mat4::IDENTITY,
            camera_to_projective: Mat4::IDENTITY,
            projective_to_camera: Mat4::IDENTITY,
            world_to_projective: Mat4::IDENTITY,
            projective_to_world: Mat4::IDENTITY,
            target_pixel_to_projective: Mat4::IDENTITY,

            frustum: Frustum::default(),

            speed_mul: 1.0,
            smooth_movement: 1.0,
            smooth_look: 0.0,
        };

        camera.update_matrices();
        camera
    }

    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.viewport = viewport;
    }
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn set_projection(&mut self, projection: CameraProjection) {
        self.projection = projection;
    }

    pub fn update_mouse(&mut self, delta: Vec2, scroll: f32) {
        self.speed_mul = (self.speed_mul + scroll * 0.05).clamp(0.0, 25.0);
        self.controller.update_mouse(&mut self.tween, delta, scroll);
        self.update_matrices();
    }

    pub fn update(&mut self, input: &InputState, delta_time: f32) {
        self.controller.update(
            &mut self.tween,
            input,
            delta_time,
            self.speed_mul,
            self.smooth_movement,
            self.smooth_look,
        );
        self.update_matrices();
    }

    pub fn update_matrices(&mut self) {
        self.world_to_camera = self.controller.view_matrix();
        self.camera_to_world = self.world_to_camera.inverse();

        self.camera_to_projective = self.projection.matrix(self.viewport.aspect_ratio());
        self.projective_to_camera = self.camera_to_projective.inverse();

        self.world_to_projective = self.camera_to_projective * self.world_to_camera;
        self.projective_to_world = self.world_to_projective.inverse();

        self.target_pixel_to_projective = self.viewport.target_pixel_to_projective();

        self.frustum = Frustum::from_matrix(self.world_to_projective);
    }

    pub fn is_point_visible(&self, point: Vec3) -> bool {
        let point_transformed = self.world_to_projective.project_point3(point);

        point_transformed.z >= 0.0
    }
}

// Functions forwarded from CameraController
impl Camera {
    pub fn position_target(&self) -> Vec3 {
        self.controller.position_target()
    }

    pub fn position(&self) -> Vec3 {
        self.controller.position()
    }

    pub fn orientation(&self) -> Vec2 {
        self.controller.orientation()
    }

    pub fn rotation(&self) -> Quat {
        self.controller.rotation()
    }

    pub fn forward(&self) -> Vec3 {
        self.controller.forward()
    }

    pub fn right(&self) -> Vec3 {
        self.controller.right()
    }

    pub fn up(&self) -> Vec3 {
        self.controller.up()
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.controller.set_position(position);
    }

    pub fn set_orientation(&mut self, orientation: Vec2) {
        self.controller.set_orientation(orientation);
    }

    pub fn view_angle(&self) -> Vec2 {
        self.controller.view_angle()
    }

    pub fn get_look_angle(&self, pos: Vec3) -> Vec2 {
        self.controller.get_look_angle(pos)
    }

    // pub fn set_rotation(&mut self, rotation: Quat) {
    //     self.controller.set_rotation(rotation);
    // }
    //
    // pub fn look_at(&mut self, target: Vec3) {
    //     self.controller.look_at(target);
    // }
}

impl View for Camera {
    fn viewport(&self) -> Viewport {
        self.viewport.clone()
    }

    fn subscribed_views(&self) -> RenderStageSubscriptions {
        RenderStageSubscriptions::all()
    }

    fn name(&self) -> String {
        "Camera".to_string()
    }

    fn update_extern(&self, x: &mut crate::tfx::externs::View) {
        x.world_to_camera = self.world_to_camera;
        x.camera_to_projective = self.camera_to_projective;

        x.derive_matrices(&self.viewport);

        // Only known values are (0, 1, 0, 0) and (0, 3.428143, 0, 0)
        x.view_miscellaneous = Vec4::new(0., 1., 0., 0.);
    }
}
