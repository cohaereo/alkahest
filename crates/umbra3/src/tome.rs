use std::ptr::null_mut;

use glam::Vec3;
use umbra3_sys::{
    Umbra_Tome, Umbra_TomeLoader_freeTome, Umbra_TomeLoader_loadFromBuffer, Umbra_Vector3,
};

pub struct Tome(pub(crate) *const Umbra_Tome);

// Tomes are immutable data, so they can be safely sent between threads.
unsafe impl Send for Tome {}
unsafe impl Sync for Tome {}

impl Tome {
    pub fn load_from_buffer(data: &[u8]) -> Self {
        // TODO(cohae): Error checking
        Self(unsafe { Umbra_TomeLoader_loadFromBuffer(data.as_ptr(), data.len(), null_mut()) })
    }

    pub fn get_cluster_bounds(&self, index: i32) -> (Vec3, Vec3) {
        let mut min = Umbra_Vector3 { v: [0f32; 3] };
        let mut max = Umbra_Vector3 { v: [0f32; 3] };
        unsafe {
            (*self.0).getClusterBounds(index, &mut min, &mut max);
        }

        (Vec3::from(min.v), Vec3::from(max.v))
    }

    pub fn get_cluster_count(&self) -> i32 {
        unsafe { (*self.0).getClusterCount() }
    }
}

impl Drop for Tome {
    fn drop(&mut self) {
        unsafe {
            Umbra_TomeLoader_freeTome(self.0, null_mut());
        }
    }
}
