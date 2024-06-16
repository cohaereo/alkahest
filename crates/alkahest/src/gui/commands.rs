use alkahest_data::entity::{SDynamicModel, SEntity};
use alkahest_pm::package_manager;
use alkahest_renderer::{
    ecs::{transform::Transform, Scene},
    renderer::RendererShared,
};
use anyhow::Context;
use destiny_pkg::TagHash;
use glam::Vec3;
use itertools::Itertools;
use tiger_parse::TigerReadable;

use crate::gui::console::{load_entity, load_entity_model};

pub fn load_pkg_entities(
    pkg_name: &str,
    renderer: RendererShared,
    scene: &mut Scene,
) -> anyhow::Result<()> {
    let entity_hashes = package_manager()
        .package_entry_index
        .iter()
        .filter(|(i, e)| package_manager().package_paths[*i].name.contains(pkg_name))
        .flat_map(|(pkg_id, entries)| {
            entries
                .iter()
                .enumerate()
                .filter(|(_, e)| Some(e.reference) == SEntity::ID)
                // .filter(|(_, e)| Some(e.reference) == SDynamicModel::ID)
                .map(|(i, e)| TagHash::new(*pkg_id, i as u16))
        })
        .collect_vec();

    let width = (entity_hashes.len() as f32).sqrt().ceil() as usize;
    for (index, &hash) in entity_hashes.iter().enumerate() {
        let pos = (index % width, index / width);
        let pos = Vec3::new(pos.0 as f32, pos.1 as f32, 0.0) * 10.0;

        let transform = Transform {
            translation: pos,
            ..Default::default()
        };
        match load_entity(hash.into(), transform, &renderer) {
            Ok(er) => {
                scene.spawn(er);
                info!("Entity spawned");
            }
            Err(e) => {} // error!("Failed to load entity {hash}: {e:?}"),
        }
    }

    Ok(())
}
