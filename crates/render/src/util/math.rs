use std::arch::x86_64::{_mm_rsqrt_ps, _mm_sqrt_ps};

use glam::{Vec3, Vec4};

pub trait Vec3Ext {
    // Copies the given vector with the vertical (Z) component set to 0
    fn flatten(&self) -> Vec3;
}

impl Vec3Ext for Vec3 {
    fn flatten(&self) -> Vec3 {
        Vec3::new(self.x, self.y, 0.0)
    }
}

pub trait Vec4Ext {
    fn sqrt(&self) -> Vec4;
    fn rsqrt(&self) -> Vec4;
}

impl Vec4Ext for Vec4 {
    fn sqrt(&self) -> Vec4 {
        unsafe { Vec4::from(_mm_sqrt_ps((*self).into())) }
    }

    fn rsqrt(&self) -> Vec4 {
        unsafe { Vec4::from(_mm_rsqrt_ps((*self).into())) }
    }
}
