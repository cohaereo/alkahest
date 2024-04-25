#[derive(Debug, Clone)]
pub struct Viewport {
    pub origin: glam::UVec2,
    pub size: glam::UVec2,
}

impl Viewport {
    pub fn aspect_ratio(&self) -> f32 {
        self.size.x as f32 / self.size.y as f32
    }
}
