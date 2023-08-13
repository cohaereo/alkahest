use binrw::BinRead;
use bytemuck::{Pod, Zeroable};
use std::fmt::{Debug, Formatter, Write};

#[derive(BinRead, Copy, Clone, PartialEq)]
pub struct DestinyHash(pub u32);

impl From<DestinyHash> for u32 {
    fn from(value: DestinyHash) -> Self {
        value.0
    }
}

impl From<u32> for DestinyHash {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl DestinyHash {
    pub fn is_none(&self) -> bool {
        self.0 == 0x811c9dc5
    }
}

impl Debug for DestinyHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_none() {
            f.write_fmt(format_args!("DestinyHash(NONE)"))
        } else {
            f.write_str("DestinyHash(")?;

            self.0.fmt(f)?;

            f.write_char(')')
        }
    }
}

#[derive(BinRead, Debug, Clone)]
pub struct IVector2 {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Default, Pod, Zeroable)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Default, Pod, Zeroable)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Default, Pod, Zeroable)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Debug for Vector2 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Vector2(x={:.3}, y={:.3})", self.x, self.y))
    }
}

impl Debug for Vector3 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Vector3(x={:.3}, y={:.3}, z={:.3})",
            self.x, self.y, self.z,
        ))
    }
}

impl Debug for Vector4 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Vector4(x={}, y={}, z={}, w={})",
            self.x, self.y, self.z, self.w
        ))
    }
}

impl<T> From<[T; 2]> for Vector2
where
    T: DecodeFloat + Copy,
{
    fn from(value: [T; 2]) -> Self {
        Self {
            x: value[0].decode_float(),
            y: value[1].decode_float(),
        }
    }
}

impl<T> From<[T; 3]> for Vector3
where
    T: DecodeFloat + Copy,
{
    fn from(value: [T; 3]) -> Self {
        Self {
            x: value[0].decode_float(),
            y: value[1].decode_float(),
            z: value[2].decode_float(),
        }
    }
}

impl<T> From<[T; 4]> for Vector4
where
    T: DecodeFloat + Copy,
{
    fn from(value: [T; 4]) -> Self {
        Self {
            x: value[0].decode_float(),
            y: value[1].decode_float(),
            z: value[2].decode_float(),
            w: value[3].decode_float(),
        }
    }
}

pub trait DecodeFloat {
    fn decode_float(self) -> f32;
}

impl DecodeFloat for f32 {
    fn decode_float(self) -> f32 {
        self
    }
}

macro_rules! impl_decode_float {
    ($t:ty) => {
        impl DecodeFloat for $t {
            fn decode_float(self) -> f32 {
                self as f32 / <$t>::MAX as f32
            }
        }
    };
    ($($t:ty),+) => {
        $(impl_decode_float!($t);)*
    };
}

impl_decode_float!(u8, u16);
impl_decode_float!(i8, i16);
