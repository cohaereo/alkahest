use alkahest_data::occlusion::AABB;
use glam::Vec3;
use hecs::EntityRef;


use self::transform::Transform;

pub mod components;
pub mod dynamic_geometry;
pub mod light;
pub mod resources;
pub mod static_geometry;
pub mod tags;
pub mod terrain;
pub mod transform;
pub mod utility;

pub type Scene = hecs::World;

pub fn resolve_aabb(e: EntityRef<'_>) -> Option<AABB> {
    if let Some(ruler) = e.get::<&utility::Ruler>() {
        return Some(AABB::from_points([ruler.start, ruler.end]));
    }

    // if let Some(si) = e.get::<&components::StaticInstances>() {
    //     let points =
    //         si.0.occlusion_bounds
    //             .iter()
    //             .flat_map(|v| [v.min, v.max])
    //             .collect_vec();
    //     return Some(AABB::from_points(points));
    // }

    if let Some(transform) = e.get::<&Transform>() {
        let radius = transform.radius();
        if radius.is_normal() {
            return Some(AABB::from_points([
                transform.translation - Vec3::ONE * radius,
                transform.translation + Vec3::ONE * radius,
            ]));
        }
    }

    None
}
