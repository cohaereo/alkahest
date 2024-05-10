pub enum CameraProjection {
    Perspective {
        /// Field of view in degrees
        fov: f32,
        near: f32,
    },
    PerspectiveBounded {
        /// Field of view in degrees
        fov: f32,
        near: f32,
        far: f32,
    },
    Orthographic {
        extents: glam::Vec3,
    },
}

impl CameraProjection {
    pub fn perspective(fov: f32, near: f32) -> Self {
        Self::Perspective { fov, near }
    }
    
    pub fn perspective_bounded(fov: f32, near: f32, far: f32) -> Self {
        Self::PerspectiveBounded { fov, near, far }
    }

    pub fn orthographic(extents: glam::Vec3) -> Self {
        Self::Orthographic { extents }
    }

    pub fn matrix(&self, aspect: f32) -> glam::Mat4 {
        match self {
            Self::Perspective { fov, near } => {
                glam::Mat4::perspective_infinite_reverse_rh(fov.to_radians(), aspect, *near)
            }
            Self::PerspectiveBounded { fov, near, far } => {
                glam::Mat4::perspective_rh(fov.to_radians(), aspect, *near, *far)
            }
            Self::Orthographic { extents } => glam::Mat4::orthographic_rh(
                -extents.x, extents.x, -extents.y, extents.y, -extents.z, extents.z,
            ),
        }
    }
}
