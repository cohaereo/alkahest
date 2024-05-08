use glam::Vec4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color(pub Vec4);

impl Color {
    pub const BLACK: Self = Self(Vec4::ZERO);
    pub const WHITE: Self = Self(Vec4::ONE);

    pub const RED: Self = Self(Vec4::new(1.0, 0.0, 0.0, 1.0));
    pub const GREEN: Self = Self(Vec4::new(0.0, 1.0, 0.0, 1.0));
    pub const BLUE: Self = Self(Vec4::new(0.0, 0.0, 1.0, 1.0));

    pub const YELLOW: Self = Self(Vec4::new(1.0, 1.0, 0.0, 1.0));
    pub const CYAN: Self = Self(Vec4::new(0.0, 1.0, 1.0, 1.0));
    pub const MAGENTA: Self = Self(Vec4::new(1.0, 0.0, 1.0, 1.0));

    pub const ORANGE: Self = Self(Vec4::new(1.0, 0.5, 0.0, 1.0));
    pub const PINK: Self = Self(Vec4::new(1.0, 0.0, 0.5, 1.0));
    pub const PURPLE: Self = Self(Vec4::new(0.5, 0.0, 1.0, 1.0));
    pub const TURQUOISE: Self = Self(Vec4::new(0.0, 1.0, 0.5, 1.0));

    pub const DARK_RED: Self = Self(Vec4::new(0.5, 0.0, 0.0, 1.0));
    pub const DARK_GREEN: Self = Self(Vec4::new(0.0, 0.5, 0.0, 1.0));
    pub const DARK_BLUE: Self = Self(Vec4::new(0.0, 0.0, 0.5, 1.0));

    pub const DARK_YELLOW: Self = Self(Vec4::new(0.5, 0.5, 0.0, 1.0));
    pub const DARK_CYAN: Self = Self(Vec4::new(0.0, 0.5, 0.5, 1.0));
    pub const DARK_MAGENTA: Self = Self(Vec4::new(0.5, 0.0, 0.5, 1.0));

    pub const GRAY: Self = Self(Vec4::new(0.5, 0.5, 0.5, 1.0));

    pub fn r(self) -> f32 {
        self.0.x
    }
    
    pub fn g(self) -> f32 {
        self.0.y
    }
    
    pub fn b(self) -> f32 {
        self.0.z
    }
    
    pub fn a(self) -> f32 {
        self.0.w
    }
    
    pub fn with_alpha(self, alpha: f32) -> Self {
        Self(Vec4::new(self.0.x, self.0.y, self.0.z, alpha))
    }

    pub fn is_opaque(self) -> bool {
        self.0.w >= 1.0
    }

    pub fn text_color_for_background(&self) -> Self {
        let luma = 0.2126 * self.r() + 0.7152 * self.g() + 0.0722 * self.b();

        if luma > 0.5 {
            Self::BLACK
        } else {
            Self::WHITE
        }
    }
}

impl From<[u8; 4]> for Color {
    fn from(v: [u8; 4]) -> Self {
        Self(Vec4::new(
            v[0] as f32 / 255.0,
            v[1] as f32 / 255.0,
            v[2] as f32 / 255.0,
            v[3] as f32 / 255.0,
        ))
    }
}

impl From<[f32; 4]> for Color {
    fn from(v: [f32; 4]) -> Self {
        Self(Vec4::new(v[0], v[1], v[2], v[3]))
    }
}

impl From<Vec4> for Color {
    fn from(v: Vec4) -> Self {
        Self(v)
    }
}

impl Into<Vec4> for Color {
    fn into(self) -> Vec4 {
        self.0
    }
}
