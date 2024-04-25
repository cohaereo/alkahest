use glam::Vec3;

pub struct Ruler {
    pub start: Vec3,
    pub end: Vec3,
    pub color: [u8; 3],
    pub rainbow: bool,
    pub scale: f32,
    pub marker_interval: f32,
    pub show_individual_axis: bool,
}

impl Default for Ruler {
    fn default() -> Self {
        Self {
            start: Vec3::ZERO,
            end: Vec3::ZERO,
            color: [255, 255, 255],
            rainbow: false,
            scale: 1.0,
            marker_interval: 0.0,
            show_individual_axis: false,
        }
    }
}

impl Ruler {
    pub fn length(&self) -> f32 {
        (self.start - self.end).length()
    }

    pub fn direction(&self) -> Vec3 {
        (self.end - self.start).normalize()
    }
}

pub struct Sphere {
    pub detail: u8,
    pub color: [u8; 4],
    pub rainbow: bool,
}

impl Default for Sphere {
    fn default() -> Self {
        Self {
            detail: 4,
            color: [255, 255, 255, 80],
            rainbow: false,
        }
    }
}

pub struct Beacon {
    pub color: [u8; 3],
    pub freq: f32,
    pub distance: f32,
    pub travel_time: f32,
}

impl Default for Beacon {
    fn default() -> Self {
        Self {
            color: [255, 255, 255],
            freq: 1.0,
            distance: 0.5,
            travel_time: 0.7,
        }
    }
}
