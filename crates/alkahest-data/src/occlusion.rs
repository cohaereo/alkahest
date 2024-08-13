use bevy_ecs::system::IntoSystem;
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
    pub bb: Aabb,
    pub unk20: [u32; 4],
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(bevy_ecs::component::Component))]
#[repr(C, align(16))]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const INFINITE: Self = Self {
        min: Vec3::splat(f32::NEG_INFINITY),
        max: Vec3::splat(f32::INFINITY),
    };

    pub const ZERO: Self = Self {
        min: Vec3::ZERO,
        max: Vec3::ZERO,
    };

    pub fn from_center_extents(center: Vec3, extents: Vec3) -> Self {
        Self {
            min: center - extents,
            max: center + extents,
        }
    }

    /// Creates an AABB from a collection of OBBs
    pub fn from_obbs(obbs: impl IntoIterator<Item = (Mat4, Aabb)>) -> Aabb {
        // gather all corners
        let points = obbs
            .into_iter()
            .flat_map(|(transform, aabb)| {
                aabb.corners()
                    .map(|corner| transform.transform_point3(corner))
            })
            .collect::<Vec<_>>();

        Aabb::from_points(points)
    }

    pub fn from_projection_matrix(local_to_world: Mat4) -> Aabb {
        let points = [
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ];

        Aabb::from_points(points.iter().map(|p| local_to_world.project_point3(*p)))
    }

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

    pub fn from_points(points: impl IntoIterator<Item = Vec3>) -> Aabb {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for point in points.into_iter() {
            min = min.min(point);
            max = max.max(point);
        }

        Self { min, max }
    }

    pub fn corners(&self) -> [Vec3; 8] {
        [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ]
    }

    /// Translates world space AABB to local space, taking a local -> world transform
    pub fn untransform(&self, transform: Mat4) -> Self {
        let mat = transform.inverse();
        let corners = self.corners();
        Self::from_points(corners.iter().map(|corner| mat.transform_point3(*corner)))
    }
}

impl TigerReadable for Aabb {
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
