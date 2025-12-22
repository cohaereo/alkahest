use alkahest_data::tfx::common::AxisAlignedBBox;
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};

use crate::visibility::frustum::Frustum;

// A simple camera controller (X forward, Z up)
pub struct Camera {
    pub position: Vec3,
    pub rotation: Quat,

    pub projection: CameraProjection,
    pub near: f32,
    pub far: f32,

    pub fov_y: f32,
    pub max_ortho_width: f32,

    pub aspect_ratio: f32,
    pub culling_frustum: Frustum,

    pub local_to_camera: Mat4,
    pub camera_to_projective: Mat4,
    pub local_to_projective: Mat4,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            // projection: CameraProjection::perspective(90.0, Self::NEAR, Self::FAR),
            projection: CameraProjection::Perspective,
            near: Self::NEAR,
            far: Self::FAR,
            fov_y: 90.0,
            max_ortho_width: 2.0,
            aspect_ratio: 16. / 9.,
            culling_frustum: Frustum::default(),
            local_to_camera: Mat4::IDENTITY,
            camera_to_projective: Mat4::IDENTITY,
            local_to_projective: Mat4::IDENTITY,
        }
    }
}

impl Camera {
    pub const NEAR: f32 = 0.15;
    pub const FAR: f32 = 50000.0;

    pub fn update(&mut self) {
        self.fov_y += 10.0;
        self.culling_frustum = Frustum::from_camera(self);
        self.fov_y -= 10.0;

        self.local_to_camera = self.view_matrix();
        self.camera_to_projective = self.projection_matrix(self.aspect_ratio);
        self.local_to_projective = self.camera_to_projective * self.local_to_camera;
    }

    pub fn view_matrix(&self) -> glam::Mat4 {
        glam::Mat4::look_at_rh(
            self.position,
            self.position + self.rotation.mul_vec3(Vec3::X),
            Vec3::Z,
        )
    }

    pub fn projection_matrix(&self, aspect_ratio: f32) -> glam::Mat4 {
        self.projection.matrix(
            aspect_ratio,
            match self.projection {
                CameraProjection::Perspective => self.fov_y,
                CameraProjection::Orthographic => self.max_ortho_width,
            },
            self.near,
            self.far,
        )
    }

    pub fn projection_matrix_standard(&self) -> glam::Mat4 {
        self.projection.matrix_standard(
            self.aspect_ratio,
            match self.projection {
                CameraProjection::Perspective => self.fov_y,
                CameraProjection::Orthographic => self.max_ortho_width,
            },
            self.near,
            self.far,
        )
    }

    pub fn forward(&self) -> Vec3 {
        self.rotation.mul_vec3(Vec3::X)
    }

    pub fn right(&self) -> Vec3 {
        self.rotation.mul_vec3(-Vec3::Y)
    }

    pub fn up(&self) -> Vec3 {
        self.rotation.mul_vec3(Vec3::Z)
    }

    pub fn is_visible(&self, aabb: &AxisAlignedBBox) -> bool {
        if !self.culling_frustum.aabb_intersecting(aabb) {
            return false;
        }

        // Project the AABB corners to check how big they appear on screen
        let corners = aabb.points();
        let mut min_ndc = Vec3::splat(f32::MAX);
        let mut max_ndc = Vec3::splat(f32::MIN);
        for corner in &corners {
            let world_pos = corner.extend(1.0);
            let clip_pos = self.local_to_projective * world_pos;
            let ndc_pos = clip_pos.truncate() / clip_pos.w;

            min_ndc = min_ndc.min(ndc_pos);
            max_ndc = max_ndc.max(ndc_pos);
        }

        // If the projected size is too small, consider it not visible
        let ndc_size = max_ndc - min_ndc;
        let screen_size_threshold = 0.01; // Adjust this threshold as needed
        if ndc_size.x < screen_size_threshold && ndc_size.y < screen_size_threshold {
            return false;
        }

        true
    }
}

#[derive(Clone)]
pub enum CameraProjection {
    Perspective,
    Orthographic,
}

impl CameraProjection {
    /// Generates a projection matrix compatible with Tiger's projection matrices
    pub fn matrix(&self, aspect: f32, fov_or_max_width: f32, near: f32, far: f32) -> glam::Mat4 {
        match self {
            Self::Perspective => {
                let f = 1.0 / f32::tan(0.5 * fov_or_max_width.to_radians());
                let far = (1. / far) * near;
                Mat4::from_cols(
                    Vec4::new(f / aspect, 0.0, 0.0, 0.0),
                    Vec4::new(0.0, f, 0.0, 0.0),
                    Vec4::new(0.0, 0.0, far, -1.0),
                    Vec4::new(0.0, 0.0, near, 0.0),
                )
            }
            Self::Orthographic => {
                let extents = Vec2::new(fov_or_max_width, fov_or_max_width / aspect) * 0.5;
                glam::Mat4::orthographic_rh(-extents.x, extents.x, -extents.y, extents.y, far, near)
            }
        }
    }

    /// Standard matrix as used by glam and other common graphics libraries
    pub fn matrix_standard(
        &self,
        aspect: f32,
        fov_or_max_width: f32,
        near: f32,
        far: f32,
    ) -> glam::Mat4 {
        match self {
            Self::Perspective => {
                Mat4::perspective_rh(fov_or_max_width.to_radians(), aspect, near, far)
            }
            _ => self.matrix(aspect, fov_or_max_width, near, far),
        }
    }
}
