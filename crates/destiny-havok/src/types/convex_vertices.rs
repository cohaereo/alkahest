use binrw::binread;
use glam::{Vec3, Vec4};

use super::hkArrayIndex;

#[binread]
#[derive(Debug)]
pub struct hkpConvexVerticesShape {
    pub unk0: [u32; 12],

    #[br(map = Vec4::from_array)]
    pub aabb_half_extents: Vec4,
    #[br(map = Vec4::from_array)]
    pub aabb_center: Vec4,

    pub rotated_vertices: hkArrayIndex,

    pub unk70: [u64; 2],

    pub plane_equations: hkArrayIndex,
    // pub connectivity: hkpConvexVerticesConnectivity,
}

#[binread]
#[derive(Debug)]
pub struct hkFourTransposedPoints {
    pub rows: [[f32; 4]; 3],
}

impl hkFourTransposedPoints {
    /// Transpose the vertices from 3x4 to 4x3
    pub fn transpose(&self) -> [Vec3; 4] {
        [
            Vec3::new(self.rows[0][0], self.rows[1][0], self.rows[2][0]),
            Vec3::new(self.rows[0][1], self.rows[1][1], self.rows[2][1]),
            Vec3::new(self.rows[0][2], self.rows[1][2], self.rows[2][2]),
            Vec3::new(self.rows[0][3], self.rows[1][3], self.rows[2][3]),
        ]
    }
}
