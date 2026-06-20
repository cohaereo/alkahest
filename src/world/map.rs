use std::io::{Cursor, Seek};

use alkahest_data::{
    activity::{SActivity, SUnk80808948},
    map::{ComponentData, SBubbleParent, SMapNodeTable},
    pattern::SComponent,
};
use anyhow::Context;
use glam::Vec4Swizzles;
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{TagHash, package_manager};

use crate::world::{pattern::spawn_pattern, transform::Transform};

pub fn load_map_into_world(taghash: TagHash, world: &mut hecs::World) -> anyhow::Result<()> {
    info!("Loading map {taghash}");
    let parent = package_manager()
        .read_tag_struct::<SBubbleParent>(taghash)
        .context("Failed to read SBubbleParent")?;
    for resources in &parent.definition.containers {
        for datatable_hash in &resources.data_tables {
            load_nodetable_into_world(*datatable_hash, world)?;
        }
    }

    Ok(())
}

pub fn load_activity_phase_into_world(
    phase: &SUnk80808948,
    world: &mut hecs::World,
) -> anyhow::Result<()> {
    for res in &phase.unk_entity_reference.unk18.entity_resources {
        let component_data = package_manager()
            .read_tag(res.entity_resource)
            .context("Failed to read component data")?;
        let mut component_data = Cursor::new(component_data);

        let component =
            SComponent::read_ds(&mut component_data).context("Failed to read SComponent")?;

        if component.definition.resource_type == 0x808092D8 {
            component_data.seek(std::io::SeekFrom::Start(component.definition.offset + 0x84))?;
            let nodetable_hash = TagHash::read_ds(&mut component_data)?;
            load_nodetable_into_world(nodetable_hash, world)?;
        }
    }

    Ok(())
}

pub fn load_activity_for_map_into_world(
    activity_hash: impl Into<TagHash>,
    bubble_hash: u32,
    world: &mut hecs::World,
) -> anyhow::Result<()> {
    let activity: SActivity = package_manager().read_tag_struct(activity_hash.into())?;
    let activity_map = &activity
        .unk50
        .iter()
        .find(|b| b.bubble_name == bubble_hash)
        .context("Map index out of range")?;

    for unk in &activity_map.unk18 {
        if let Err(e) = load_activity_phase_into_world(unk, world) {
            error!(
                "Activity phase load for {} failed: {e}",
                unk.unk_entity_reference.taghash()
            )
        }
    }

    Ok(())
}

pub fn load_nodetable_into_world(
    table_hash: TagHash,
    world: &mut hecs::World,
) -> anyhow::Result<()> {
    let table: SMapNodeTable = package_manager().read_tag_struct(table_hash)?;
    for node in table.nodes {
        let transform = Transform::new(
            node.translation.xyz(),
            node.rotation,
            node.translation.www(),
        );

        for (i, data) in node.component_data.iter().enumerate() {
            if let ComponentData::Unknown { class, offset, .. } = data {
                debug!(
                    "Unknown dynamic component data class: {:08X} (#{}) in {table_hash} at \
                     offset: {:#X}",
                    class, i, offset
                );
            }
        }

        if node.entity.is_none() {
            anyhow::bail!(
                "Map data table node with world id {} has no entity. This shouldn't be possible!",
                node.world_id
            );
        }

        if let Err(e) = spawn_pattern(
            world,
            node.entity.hash32(),
            Some(&node.component_data),
            Some(transform),
        ) {
            error!("Failed to load entity: {:?}", e);
        }
    }

    Ok(())
}
