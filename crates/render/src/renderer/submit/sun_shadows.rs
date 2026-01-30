use crate::Renderer;

impl Renderer {
    pub const NUM_CASCADES: usize = 4;
    pub const MAX_CASCADE_DISTANCE: f32 = 600.0;
    pub const CASCADE_DISTANCES: [f32; Self::NUM_CASCADES] =
        [10.0, 30.0, 100.0, Self::MAX_CASCADE_DISTANCE];

    pub fn get_cascade_distance_range(index: usize) -> (f32, f32) {
        let z_near = if index == 0 {
            0.05
        } else {
            Self::CASCADE_DISTANCES[index - 1]
        };
        let z_far = Self::CASCADE_DISTANCES[index];
        (z_near, z_far)
    }
}
