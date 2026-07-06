use glam::Vec3;
use tiger_pkg::TagHash;

use crate::world::{object::ObjectChannels, transform::Transform};

#[cfg(feature = "wwise")]
pub mod audio;
pub mod map;
pub mod object;
pub mod pattern;
pub mod render_objects;
pub mod sequencer;
pub mod shadowmap;
pub mod transform;

#[allow(unused)]
pub struct UnimplementedTigerComponent {
    pub class_id: u32,
    pub hash: TagHash,
    pub name: Option<String>,
}

pub struct UnimplementedTigerComponents(pub Vec<UnimplementedTigerComponent>);

#[profiling::function]
pub fn s_update_object_channels(world: &hecs::World) {
    for (_entity, (transform, object_channels)) in world
        .query::<(Option<&Transform>, &mut ObjectChannels)>()
        .iter()
    {
        object_channels.reset_usage_counters();
        object_channels.set_by_name(
            "device_position",
            transform.map_or(Vec3::ZERO, |t| t.translation).extend(1.0),
        );
        object_channels.set_by_name(
            "interpolated_world_position",
            transform.map_or(Vec3::ZERO, |t| t.translation).extend(1.0),
        );
        object_channels.set_by_id(
            0x8a6c82c5,
            transform.map_or(Vec3::ZERO, |t| t.translation).extend(1.0),
        );
    }
}
