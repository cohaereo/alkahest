pub enum CameraProjection {
    Perspective {
        /// Field of view in degrees
        fov: f32,
        near: f32,
    },
    Orthographic {
        extents: glam::Vec3,
    },
}

impl CameraProjection {
    pub fn perspective(fov: f32, near: f32) -> Self {
        Self::Perspective { fov, near }
    }

    pub fn orthographic(extents: glam::Vec3) -> Self {
        Self::Orthographic { extents }
    }

    pub fn matrix(&self, aspect: f32) -> glam::Mat4 {
        match self {
            Self::Perspective { fov, near } => {
                glam::Mat4::perspective_infinite_reverse_rh(fov.to_radians(), aspect, *near)
            }
            Self::Orthographic { extents } => glam::Mat4::orthographic_rh(
                -extents.x, extents.x, -extents.y, extents.y, -extents.z, extents.z,
            ),
        }
    }
}
