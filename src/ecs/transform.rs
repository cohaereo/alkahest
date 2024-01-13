use bitflags::bitflags;
use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(C, align(16))]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,

    pub flags: TransformFlags,
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
            flags: TransformFlags::default(),
        }
    }

    pub fn to_mat4(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// If scale is radius, returns the radius, otherwise returns NaN
    pub fn radius(&self) -> f32 {
        if self.flags.contains(TransformFlags::SCALE_IS_RADIUS) {
            self.scale.x
        } else {
            f32::NAN
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            flags: TransformFlags::default(),
        }
    }
}

bitflags! {
    #[derive(Default, Debug, Copy, Clone, PartialEq)]
    pub struct TransformFlags: u32 {
        const IGNORE_TRANSLATION = (1 << 0);
        const IGNORE_ROTATION = (1 << 1);
        const IGNORE_SCALE = (1 << 2);

        const SCALE_IS_RADIUS = (1 << 3);
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct OriginalTransform(pub Transform);
