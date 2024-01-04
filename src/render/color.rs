use glam::{Vec3, Vec4};

#[derive(Debug, Copy, Clone)]
pub struct Color(pub Vec4);

impl From<Vec4> for Color {
    fn from(value: Vec4) -> Self {
        Color(value)
    }
}

impl From<[f32; 3]> for Color {
    fn from(value: [f32; 3]) -> Self {
        Color(Vec3::from_array(value).extend(1.0))
    }
}

impl From<[f32; 4]> for Color {
    fn from(value: [f32; 4]) -> Self {
        Color(value.into())
    }
}

impl From<[u8; 3]> for Color {
    fn from(value: [u8; 3]) -> Self {
        Color(
            [
                value[0] as f32 / 255.0,
                value[1] as f32 / 255.0,
                value[2] as f32 / 255.0,
                1.0,
            ]
            .into(),
        )
    }
}

impl From<[u8; 4]> for Color {
    fn from(value: [u8; 4]) -> Self {
        Color(
            [
                value[0] as f32 / 255.0,
                value[1] as f32 / 255.0,
                value[2] as f32 / 255.0,
                value[3] as f32 / 255.0,
            ]
            .into(),
        )
    }
}
