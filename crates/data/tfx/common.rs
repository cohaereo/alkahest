use std::iter::Sum;

use glam::{vec3, Quat, Vec3, Vec4, Vec4Swizzles};
use tiger_parse::tiger_type;

#[tiger_type(id = 0x80809F4F)]
#[derive(Clone, Debug, Copy)]
pub struct SRotationTranslation {
    pub rotation: Quat,
    pub translation: Vec4,
}

impl SRotationTranslation {
    pub const IDENTITY: Self = Self {
        rotation: Quat::IDENTITY,
        translation: Vec4::ZERO,
    };
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808093B1, size = 0x18)]
pub struct SOcclusionBounds {
    pub file_size: u64,
    pub bounds: Vec<SObjectOcclusionBounds>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808093B3, size = 0x30)]
pub struct SObjectOcclusionBounds {
    pub bb: AxisAlignedBBox,
    pub unk20: [u32; 4],
}

#[tiger_type]
#[derive(Debug, Clone, Copy)]
pub struct AxisAlignedBBox {
    pub min: Vec4,
    pub max: Vec4,
}

impl AxisAlignedBBox {
    pub const NONE: Self = Self {
        min: Vec4::new(f32::MAX, f32::MAX, f32::MAX, f32::MAX),
        max: Vec4::new(f32::MIN, f32::MIN, f32::MIN, f32::MIN),
    };

    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }

    pub fn from_center_extents(center: Vec3, extents: Vec3) -> Self {
        Self {
            min: vec3(
                center.x - extents.x / 2.0,
                center.y - extents.y / 2.0,
                center.z - extents.z / 2.0,
            )
            .extend(0.0),
            max: vec3(
                center.x + extents.x / 2.0,
                center.y + extents.y / 2.0,
                center.z + extents.z / 2.0,
            )
            .extend(0.0),
        }
    }

    pub fn from_points(points: &[Vec3]) -> Self {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for &point in points {
            min = min.min(point);
            max = max.max(point);
        }

        Self {
            min: min.extend(1.0),
            max: max.extend(1.0),
        }
    }

    pub fn points(&self) -> [Vec3; 8] {
        [
            vec3(self.min.x, self.min.y, self.min.z),
            vec3(self.min.x, self.min.y, self.max.z),
            vec3(self.min.x, self.max.y, self.min.z),
            vec3(self.min.x, self.max.y, self.max.z),
            vec3(self.max.x, self.min.y, self.min.z),
            vec3(self.max.x, self.min.y, self.max.z),
            vec3(self.max.x, self.max.y, self.min.z),
            vec3(self.max.x, self.max.y, self.max.z),
        ]
    }

    pub fn transformed(&self, transform: glam::Mat4) -> Self {
        let points = self.points();
        let transformed_points: Vec<Vec3> = points
            .iter()
            .map(|&point| transform.transform_point3(point))
            .collect();
        Self::from_points(&transformed_points)
    }

    pub fn extents(&self) -> Vec3 {
        (self.max - self.min).xyz()
    }

    pub fn center(&self) -> Vec3 {
        ((self.min + self.max) / 2.0).xyz()
    }

    pub fn radius(&self) -> f32 {
        self.extents().length() / 2.0
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn contains(&self, point: Vec3) -> bool {
        self.min.x <= point.x
            && point.x <= self.max.x
            && self.min.y <= point.y
            && point.y <= self.max.y
            && self.min.z <= point.z
            && point.z <= self.max.z
    }

    pub fn expand(&self, amount: Vec3) -> Self {
        Self {
            min: self.min - amount.extend(0.0),
            max: self.max + amount.extend(0.0),
        }
    }
}

impl Sum for AxisAlignedBBox {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Self::NONE, |acc, bbox| acc.union(&bbox))
    }
}
