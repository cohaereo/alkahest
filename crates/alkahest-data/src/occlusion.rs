use glam::{Mat4, Quat, Vec3};
use tiger_parse::{tiger_tag, TigerReadable};

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x808093B1, size = 0x18)]
pub struct SOcclusionBounds {
    pub file_size: u64,
    pub bounds: Vec<SObjectOcclusionBounds>,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x808093B3, size = 0x30)]
pub struct SObjectOcclusionBounds {
    pub bb: AABB,
    pub unk20: [u32; 4],
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(C, align(16))]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub const INFINITE: Self = Self {
        min: Vec3::splat(f32::NEG_INFINITY),
        max: Vec3::splat(f32::INFINITY),
    };

    // pub fn contains_point(&self, point: Vec3) -> bool {
    //     self.min.x <= point.x
    //         && self.min.y <= point.y
    //         && self.min.z <= point.z
    //         && point.x <= self.max.x
    //         && point.y <= self.max.y
    //         && point.z <= self.max.z
    // }

    pub fn contains_point_oriented(&self, point: Vec3, orientation: Quat) -> bool {
        let mut matrix =
            Mat4::from_scale_rotation_translation(self.extents(), orientation, self.center());

        matrix = matrix.inverse();

        let point_transformed = matrix.project_point3(point);

        point_transformed.cmpge(Vec3::NEG_ONE).all() && point_transformed.cmple(Vec3::ONE).all()
    }

    /// In units cubed
    pub fn volume(&self) -> f32 {
        let dimensions = self.max - self.min;
        dimensions.x * dimensions.y * dimensions.z
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) / 2.0
    }

    pub fn dimensions(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn extents(&self) -> Vec3 {
        self.dimensions() / 2.0
    }

    pub fn radius(&self) -> f32 {
        self.extents().length()
    }

    pub fn from_points(points: impl AsRef<[Vec3]>) -> AABB {
        let points = points.as_ref();

        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for point in points {
            min = min.min(*point);
            max = max.max(*point);
        }

        Self { min, max }
    }
}

impl TigerReadable for AABB {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let min = glam::Vec4::read_ds_endian(reader, endian)?;
        let max = glam::Vec4::read_ds_endian(reader, endian)?;

        Ok(Self {
            min: min.truncate(),
            max: max.truncate(),
        })
    }

    const SIZE: usize = glam::Vec4::SIZE * 2;
    const ZEROCOPY: bool = false;
}
