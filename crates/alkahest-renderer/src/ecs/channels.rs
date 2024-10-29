use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Without,
    system::{In, Query},
};
use glam::Vec4;
use rustc_hash::FxHashMap;

use crate::{renderer::RendererShared, tfx::channels::ChannelType};

use super::render::dynamic_geometry::DynamicModelComponent;

#[derive(Component)]
pub struct ObjectChannels {
    pub values: FxHashMap<u32, (Vec4, ChannelType)>,
}

// Discover channels used by dynamic objects by going over every object with a DynamicModelComponent that doesn't already have a ObjectChannels component
// If the dynamic model's techniques haven't been loaded by the asset system yet, this system will skip it and try again next time
pub fn object_channels_discovery_system(
    In(renderer): In<RendererShared>,
    mut commands: bevy_ecs::system::Commands,
    q_dynamic_model: Query<(Entity, &DynamicModelComponent), Without<ObjectChannels>>,
) {
    let assets = &renderer.data.lock().asset_manager;
    'entity: for (entity, model) in q_dynamic_model.iter() {
        let mut object_ids = FxHashMap::default();
        for t in model.techniques() {
            if let Some(technique) = assets.techniques.get(&t) {
                for (hash, channel_type) in technique.object_channel_ids() {
                    let e = object_ids.entry(hash).or_insert(ChannelType::Float);
                    *e = channel_type.pick_best_type(e.clone());
                }
            } else {
                if !t.is_none() {
                    // Technique not loaded yet, skip this object for now
                    continue 'entity;
                }
            }
        }

        // if !object_ids.is_empty() {
        //     println!(
        //         "Discovered {} object channels for model {entity}",
        //         object_ids.len()
        //     );
        // }

        commands.entity(entity).insert(ObjectChannels {
            values: object_ids
                .iter()
                .map(|(&id, ty)| (id, (Vec4::ONE, ty.clone())))
                .collect(),
        });
    }
}
