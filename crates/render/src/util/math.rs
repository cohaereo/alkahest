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
        #[cfg(target_feature = "sse")]
        unsafe {
            Vec4::from(std::arch::x86_64::_mm_sqrt_ps((*self).into()))
        }
        #[cfg(not(target_feature = "sse"))]
        {
            Vec4::new(self.x.sqrt(), self.y.sqrt(), self.z.sqrt(), self.w.sqrt())
        }
    }

    fn rsqrt(&self) -> Vec4 {
        #[cfg(target_feature = "sse")]
        unsafe {
            Vec4::from(std::arch::x86_64::_mm_rsqrt_ps((*self).into()))
        }
        #[cfg(not(target_feature = "sse"))]
        {
            Vec4::new(
                1.0 / self.x.sqrt(),
                1.0 / self.y.sqrt(),
                1.0 / self.z.sqrt(),
                1.0 / self.w.sqrt(),
            )
        }
    }
}
