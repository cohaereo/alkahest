use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translation: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn new(position: glam::Vec3, rotation: glam::Quat, scale: Vec3) -> Self {
        Self {
            translation: position,
            rotation,
            scale,
        }
    }

    pub fn local_to_world(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}
