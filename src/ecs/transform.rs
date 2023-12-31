use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    // TODO(cohae): matrix caching
    // pub world_to_local: Mat4,
    // /// The inverse of `world_to_local`
    // pub local_to_world: Mat4,
}

impl Transform {
    pub fn from_mat4(mat: Mat4) -> Transform {
        let (scale, rotation, translation) = mat.to_scale_rotation_translation();

        Transform {
            translation,
            rotation,
            scale,
        }
    }

    pub fn to_mat4(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct OriginalTransform(pub Transform);
