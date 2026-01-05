use alkahest_data::map::{ComponentData, SBubbleParent, SMapNodeTable};
use anyhow::Context;
use glam::Vec4Swizzles;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use crate::world::{pattern::spawn_pattern, transform::Transform};

pub fn load_map_into_world(taghash: TagHash, world: &mut hecs::World) -> anyhow::Result<()> {
    let parent = package_manager()
        .read_tag_struct::<SBubbleParent>(taghash)
        .context("Failed to read SBubbleParent")?;
    for resources in &parent.definition.containers {
        for datatable_hash in &resources.data_tables {
            let datatable = package_manager()
                .read_tag_struct::<SMapNodeTable>(*datatable_hash)
                .context("Failed to read SMapNodeTable")?;
            for node in datatable.nodes {
                let transform = Transform::new(
                    node.translation.xyz(),
                    node.rotation,
                    node.translation.www(),
                );

                if let Some(ComponentData::Unknown { class, offset, .. }) =
                    *node.primary_component_data
                {
                    debug!(
                        "Unknown dynamic component data class: {:08X} in {datatable_hash} at \
                         offset: {:#X}",
                        class, offset
                    );
                }

                if node.entity.is_none() {
                    anyhow::bail!(
                        "Map data table node with world id {} has no entity. This shouldn't be \
                         possible!",
                        node.world_id
                    );
                }

                if let Err(e) = spawn_pattern(
                    world,
                    node.entity.hash32(),
                    node.primary_component_data.as_ref(),
                    Some(transform),
                ) {
                    error!("Failed to load entity: {:?}", e);
                }
            }
        }
    }

    Ok(())
}
