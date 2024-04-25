use glam::Vec3;

pub trait Vec3Ext {
    fn flatten_xy(self, default: Vec3) -> Vec3;
}

impl Vec3Ext for Vec3 {
    fn flatten_xy(mut self, default: Vec3) -> Vec3 {
        self.z = 0.0;
        self.try_normalize().unwrap_or(default)
    }
}
