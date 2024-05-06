use std::ops::Mul;

use glam::{Mat4, Vec3, Vec4};

pub trait Vec3Ext {
    fn flatten_xy(self, default: Vec3) -> Vec3;
}

impl Vec3Ext for Vec3 {
    fn flatten_xy(mut self, default: Vec3) -> Vec3 {
        self.z = 0.0;
        self.try_normalize().unwrap_or(default)
    }
}

pub fn mat4_scale_translation(scale: Vec3, translation: Vec3) -> Mat4 {
    Mat4::from_cols(
        Vec4::X.mul(scale.x),
        Vec4::Y.mul(scale.y),
        Vec4::Z.mul(scale.z),
        Vec4::from((translation, 1.0)),
    )
}
