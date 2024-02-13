use alkahest_data::occlusion::AABB;
use glam::{Quat, Vec3};

pub struct Obb {
    pub center: Vec3,
    pub extents: Vec3,
    pub orientation: Quat,
}

impl From<AABB> for Obb {
    fn from(aabb: AABB) -> Self {
        Self {
            center: aabb.center(),
            extents: aabb.extents(),
            orientation: Quat::IDENTITY,
        }
    }
}

// impl OBB {
//     pub fn distance_to_point(&self, point: Vec3) -> f32 {
//         let point = self.orientation.inverse().mul_vec3(point - self.center);

//         let mut distance_squared = 0.0;

//         for i in 0..3 {
//             let mut distance_i = point[i];

//             if distance_i < -self.extents[i] {
//                 distance_i += self.extents[i];
//             } else if distance_i > self.extents[i] {
//                 distance_i -= self.extents[i];
//             } else {
//                 distance_i = 0.0;
//             }

//             distance_squared += distance_i * distance_i;
//         }

//         distance_squared.sqrt()
//     }
// }
