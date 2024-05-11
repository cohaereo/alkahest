#[derive(Debug, Clone)]
pub struct Viewport {
    pub origin: glam::UVec2,
    pub size: glam::UVec2,
}

impl Viewport {
    pub fn aspect_ratio(&self) -> f32 {
        self.size.x as f32 / self.size.y as f32
    }

    pub fn target_pixel_to_projective(&self) -> glam::Mat4 {
        glam::Mat4::from_cols_array_2d(&[
            [2.0 / self.size.x as f32, 0.0, 0.0, 0.0],
            [0.0, -2.0 / self.size.y as f32, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ])
    }
}
