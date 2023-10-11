use binrw::{BinRead, BinReaderExt};
use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec2, Vec3, Vec3A, Vec4};
use std::fmt::{Debug, Formatter, Write};

#[derive(BinRead, Copy, Clone, PartialEq)]
pub struct ResourceHash(pub u32);

impl From<ResourceHash> for u32 {
    fn from(value: ResourceHash) -> Self {
        value.0
    }
}

impl From<u32> for ResourceHash {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl ResourceHash {
    pub fn is_none(&self) -> bool {
        self.0 == 0x811c9dc5
    }
}

impl Debug for ResourceHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_none() {
            f.write_fmt(format_args!("ResourceHash(NONE)"))
        } else {
            f.write_str("ResourceHash(")?;

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

impl From<Vector2> for Vec2 {
    fn from(val: Vector2) -> Self {
        Vec2::new(val.x, val.y)
    }
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Default, Pod, Zeroable)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl From<Vector3> for Vec3 {
    fn from(val: Vector3) -> Self {
        Vec3::new(val.x, val.y, val.z)
    }
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Default, Pod, Zeroable)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl From<Vector4> for Vec4 {
    fn from(val: Vector4) -> Self {
        Vec4::new(val.x, val.y, val.z, val.w)
    }
}

impl From<Vector4> for Quat {
    fn from(val: Vector4) -> Self {
        Quat::from_xyzw(val.x, val.y, val.z, val.w)
    }
}

#[repr(C)]
#[derive(BinRead, Copy, Clone, Default, Pod, Zeroable, Debug)]
pub struct Matrix4 {
    pub row_x: Vector4,
    pub row_y: Vector4,
    pub row_z: Vector4,
    pub row_w: Vector4,
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

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Copy, Clone)]
pub struct AABB {
    pub min: Vec3A,
    pub max: Vec3A,
}

impl AABB {
    // pub fn contains_point(&self, point: Vec3) -> bool {
    //     self.min.x <= point.x
    //         && self.min.y <= point.y
    //         && self.min.z <= point.z
    //         && point.x <= self.max.x
    //         && point.y <= self.max.y
    //         && point.z <= self.max.z
    // }

    /// In units cubed
    pub fn volume(&self) -> f32 {
        let dimensions = self.max - self.min;
        dimensions.x * dimensions.y * dimensions.z
    }

    pub fn center(&self) -> Vec3 {
        ((self.min + self.max) / 2.0).into()
    }

    pub fn dimensions(&self) -> Vec3 {
        (self.max - self.min).into()
    }
}

impl BinRead for AABB {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let min: Vector4 = reader.read_type(endian)?;
        let max: Vector4 = reader.read_type(endian)?;

        Ok(Self {
            min: Vec3A::new(min.x, min.y, min.z),
            max: Vec3A::new(max.x, max.y, max.z),
        })
    }
}
