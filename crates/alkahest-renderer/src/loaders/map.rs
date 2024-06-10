use std::{
    io::{Cursor, Read, Seek, SeekFrom},
    ops::Deref,
};

use alkahest_data::{
    activity::{SActivity, SEntityResource, Unk80808cef, Unk80808e89, Unk808092d8},
    common::ResourceHash,
    decorator::SDecorator,
    entity::{SEntity, Unk808072c5, Unk8080906b, Unk80809905},
    map::{
        SBubbleParent, SCubemapVolume, SLensFlare, SLightCollection, SMapAtmosphere, SMapDataTable,
        SShadowingLight, SUnk808068d4, SUnk80806aa7, SUnk80806ef4, SUnk8080714b, SUnk80808cb7,
    },
    tfx::TfxFeatureRenderer,
    Tag, WideHash,
};
use alkahest_pm::package_manager;
use anyhow::Context;
use binrw::BinReaderExt;
use destiny_pkg::TagHash;
use glam::{Mat4, Quat, Vec3, Vec4Swizzles};
use hecs::{DynamicBundle, Entity};
use itertools::{any, multizip, Itertools};
use rustc_hash::{FxHashMap, FxHashSet};
use tiger_parse::{Endian, FnvHash, PackageManagerExt, TigerReadable};

use crate::{
    camera::CameraProjection,
    ecs::{
        common::{Icon, Label, ResourceOrigin},
        hierarchy::{Children, Parent},
        map::{CubemapVolume, MapAtmosphere},
        render::{
            decorators::DecoratorRenderer,
            dynamic_geometry::DynamicModelComponent,
            light::{LightRenderer, ShadowMapRenderer},
            static_geometry::{StaticInstance, StaticInstances, StaticModel},
            terrain::TerrainPatches,
        },
        tags::{insert_tag, EntityTag},
        transform::{OriginalTransform, Transform, TransformFlags},
        Scene,
    },
    icons::{
        ICON_ACCOUNT_CONVERT, ICON_CUBE, ICON_CUBE_OUTLINE, ICON_FLARE, ICON_IMAGE_FILTER_HDR,
        ICON_LIGHTBULB_GROUP, ICON_LIGHTBULB_ON, ICON_SHAPE, ICON_SPHERE, ICON_SPOTLIGHT_BEAM,
        ICON_TREE, ICON_WAVES, ICON_WEATHER_FOG, ICON_WEATHER_PARTLY_CLOUDY,
    },
    renderer::{Renderer, RendererShared},
    util::{scene::SceneExt, text::StringExt},
};

pub async fn load_map(
    renderer: RendererShared,
    map_hash: TagHash,
    activity_hash: Option<TagHash>,
    load_ambient_activity: bool,
) -> anyhow::Result<Scene> {
    let bubble_parent = package_manager()
        .read_tag_struct::<SBubbleParent>(map_hash)
        .context("Failed to read SBubbleParent")?;

    let mut scene = Scene::new();

    let mut data_tables = FxHashMap::<TagHash, Entity>::default();
    for map_container in &bubble_parent.child_map.map_resources {
        let parent_entity =
            scene.spawn((Label::from(format!("Map Container {}", map_container.1)),));
        for table in &map_container.data_tables {
            data_tables.insert(*table, parent_entity);
        }
    }

    for (table_hash, parent_entity) in data_tables {
        let table_data = package_manager().read_tag(table_hash).unwrap();
        let mut cur = Cursor::new(&table_data);
        let table = TigerReadable::read_ds(&mut cur)?;

        load_datatable_into_scene(
            &table,
            table_hash,
            &mut cur,
            &mut scene,
            &renderer,
            ResourceOrigin::Map,
            Some(parent_entity),
        )
        .context("Failed to load map datatable")?;
    }

    let mut activity_entrefs: Vec<(Tag<Unk80808e89>, ResourceHash, ResourceOrigin)> =
        Default::default();
    if let Some(activity_hash) = activity_hash {
        let activity: SActivity = package_manager().read_tag_struct(activity_hash)?;
        for u1 in &activity.unk50 {
            for map in &u1.map_references {
                if map.hash32() != map_hash {
                    continue;
                }

                for u2 in &u1.unk18 {
                    activity_entrefs.push((
                        u2.unk_entity_reference.clone(),
                        u2.activity_phase_name2,
                        ResourceOrigin::Activity,
                    ));
                }
            }
        }

        for u1 in &activity.unk40 {
            for u2 in &u1.unk50 {
                activity_entrefs.push((
                    u2.unk_entity_reference.clone(),
                    u2.activity_phase_name2,
                    ResourceOrigin::Activity,
                ));
            }
        }

        if load_ambient_activity {
            match package_manager().read_tag_struct::<SActivity>(activity.ambient_activity) {
                Ok(ambient_activity) => {
                    for u1 in &ambient_activity.unk50 {
                        for map in &u1.map_references {
                            if map.hash32() != map_hash {
                                continue;
                            }

                            for u2 in &u1.unk18 {
                                activity_entrefs.push((
                                    u2.unk_entity_reference.clone(),
                                    u2.activity_phase_name2,
                                    ResourceOrigin::Ambient,
                                ));
                            }

                            for u1 in &activity.unk40 {
                                for u2 in &u1.unk50 {
                                    activity_entrefs.push((
                                        u2.unk_entity_reference.clone(),
                                        u2.activity_phase_name2,
                                        ResourceOrigin::Ambient,
                                    ));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to load ambient activity {}: {e}",
                        activity.ambient_activity
                    );
                }
            }
        }
    }

    let mut entity_worldid_name_map: FxHashMap<u64, String> = Default::default();
    for (e, _, _) in &activity_entrefs {
        for resource in &e.unk18.entity_resources {
            if let Some(strings) = get_entity_labels(resource.entity_resource) {
                entity_worldid_name_map.extend(strings);
            }
        }
    }

    let _unknown_res_types: FxHashSet<u32> = Default::default();
    let mut phase_entities = FxHashMap::<ResourceHash, Entity>::default();
    for (e, phase_name2, origin) in activity_entrefs {
        let parent_entity = *phase_entities.entry(phase_name2).or_insert_with(|| {
            scene.spawn((Label::from(format!(
                "Activity Phase 0x{:08X}",
                phase_name2.0
            )),))
        });

        for resource in &e.unk18.entity_resources {
            if resource.entity_resource.is_some() {
                let data = package_manager().read_tag(resource.entity_resource)?;
                let mut cur = Cursor::new(&data);
                let res: SEntityResource = TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                let mut data_tables = FxHashSet::default();
                match res.unk18.resource_type {
                    0x808092d8 => {
                        cur.seek(SeekFrom::Start(res.unk18.offset))?;
                        let tag: Unk808092d8 =
                            TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;
                        if tag.unk84.is_some() {
                            data_tables.insert(tag.unk84);
                        }
                    }
                    0x80808cef => {
                        cur.seek(SeekFrom::Start(res.unk18.offset))?;
                        let tag: Unk80808cef =
                            TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;
                        if tag.unk58.is_some() {
                            data_tables.insert(tag.unk58);
                        }
                    }
                    u => {
                        // if !unknown_res_types.contains(&u) {
                        warn!(
                            "Unknown activity entref resource table resource type 0x{u:X} @ \
                             0x{:X} in resource table {}",
                            res.unk18.offset, resource.entity_resource
                        );

                        //     unknown_res_types.insert(u);
                        // }
                    }
                }

                let mut data_tables2 = FxHashSet::default();
                // TODO(cohae): This is a very dirty hack to find every other data table in the entityresource. We need to fully flesh out the EntityResource format first.
                // TODO(cohae): PS: gets assigned as Activity2 (A2) to keep them separate from known tables
                for b in data.chunks_exact(4) {
                    let v: [u8; 4] = b.try_into().unwrap();
                    let hash = TagHash(u32::from_le_bytes(v));

                    if hash.is_pkg_file()
                        && package_manager()
                            .get_entry(hash)
                            .map(|v| v.reference == 0x80809883)
                            .unwrap_or_default()
                        && !data_tables.contains(&hash)
                    {
                        data_tables2.insert(hash);
                    }
                }

                if !data_tables2.is_empty() {
                    let tstr = data_tables2.iter().map(|v| v.to_string()).join(", ");
                    warn!(
                        "TODO: Found {} map data tables ({}) EntityResource by brute force ({} \
                         found normally)",
                        data_tables2.len(),
                        tstr,
                        data_tables.len()
                    );
                }

                for table_hash in data_tables {
                    let data = package_manager().read_tag(table_hash)?;
                    let mut cur = Cursor::new(&data);
                    let table: SMapDataTable =
                        TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                    load_datatable_into_scene(
                        &table,
                        table_hash,
                        &mut cur,
                        &mut scene,
                        &renderer,
                        ResourceOrigin::Map,
                        Some(parent_entity),
                    )
                    .context("Failed to load activity datatable")?;
                }

                for table_hash in data_tables2 {
                    let data = package_manager().read_tag(table_hash)?;
                    let mut cur = Cursor::new(&data);
                    let table: SMapDataTable =
                        TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                    load_datatable_into_scene(
                        &table,
                        table_hash,
                        &mut cur,
                        &mut scene,
                        &renderer,
                        // cohae: yes, this means bruteforced ambient data tables will always be
                        // shown as ambient, but i don't think it matters once we fix the normal
                        // bruteforced activity tables
                        if origin == ResourceOrigin::Ambient {
                            origin
                        } else {
                            ResourceOrigin::ActivityBruteforce
                        },
                        Some(parent_entity),
                    )
                    .context("Failed to load AB atatable")?;
                }

                if origin != ResourceOrigin::Ambient {
                    for r in &res.resource_table2 {
                        if r.unk14 != 0xFFFFFFFF && r.unk0.is_some() {
                            // SEntity::ID
                            load_entity_into_scene(
                                r.unk0.hash32(),
                                &mut scene,
                                &renderer,
                                origin,
                                None,
                                Transform::default(),
                                None,
                                0,
                            )?;
                        }
                    }
                }
            } else {
                warn!("null entity resource tag in {}", resource.taghash());
            }
        }
    }

    // TODO(cohae): The persistent tag system is used exlusively for filtering, it's otherwise entirely redundant and should be replaced by components where possible
    let mut tags: Vec<(Entity, Vec<EntityTag>)> = vec![];
    for e in scene.iter() {
        let mut tag_list = vec![];
        if let Some(origin) = e.get::<&ResourceOrigin>().as_deref().cloned() {
            match origin {
                ResourceOrigin::Map => {}
                ResourceOrigin::Activity => tag_list.push(EntityTag::Activity),
                ResourceOrigin::ActivityBruteforce => tag_list.push(EntityTag::Activity),
                ResourceOrigin::Ambient => tag_list.push(EntityTag::Ambient),
            }
        }

        // TODO(cohae): Havok tags

        tags.push((e.entity(), tag_list));
    }

    for (e, tags) in tags {
        for tag in tags {
            insert_tag(&mut scene, e, tag);
        }
    }

    Ok(scene)
}

// clippy: asset system will fix this lint on it's own (i hope)
#[allow(clippy::too_many_arguments)]
fn load_datatable_into_scene<R: Read + Seek>(
    table: &SMapDataTable,
    table_hash: TagHash,
    table_data: &mut R,
    scene: &mut Scene,
    renderer: &Renderer,
    resource_origin: ResourceOrigin,
    parent_entity: Option<Entity>,
) -> anyhow::Result<()> {
    for data in table.data_entries.iter() {
        let transform = Transform {
            translation: Vec3::new(data.translation.x, data.translation.y, data.translation.z),
            rotation: data.rotation,
            scale: Vec3::splat(data.translation.w),
            ..Default::default()
        };

        match data.data_resource.resource_type {
            // D2Class_C96C8080 (placement)
            0x80806cc9 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();
                let preheader_tag: TagHash = table_data.read_le().unwrap();
                let preheader: SUnk80806ef4 =
                    package_manager().read_tag_struct(preheader_tag).unwrap();

                for s in &preheader.instances.instance_groups {
                    let mesh_tag = preheader.instances.statics[s.static_index as usize];
                    let model =
                        StaticModel::load(&mut renderer.data.lock().asset_manager, mesh_tag)
                            .context("Failed to load StaticModel")?;

                    let transforms = &preheader.instances.transforms
                        [s.instance_start as usize..(s.instance_start + s.instance_count) as usize];

                    let parent = spawn_data_entity(scene, (), parent_entity);
                    let mut instances = vec![];

                    for transform in transforms.iter() {
                        let transform = Transform {
                            translation: transform.translation,
                            rotation: transform.rotation,
                            scale: Vec3::splat(transform.scale.x),
                            flags: TransformFlags::empty(),
                        };

                        let entity = scene.spawn((
                            Icon(ICON_CUBE_OUTLINE),
                            Label::from("Static Instance"),
                            OriginalTransform(transform),
                            transform,
                            StaticInstance,
                            Parent(parent),
                        ));
                        instances.push(entity);
                    }

                    scene.insert(
                        parent,
                        (
                            Icon(ICON_SHAPE),
                            Label::from(format!("Static Instances {mesh_tag}")),
                            StaticInstances::new(renderer.gpu.clone(), model, instances.len())?,
                            Children::from_slice(&instances),
                            TfxFeatureRenderer::StaticObjects,
                            resource_origin,
                        ),
                    )?;
                }
            }
            // D2Class_7D6C8080 (terrain)
            0x80806c7d => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset))
                    .unwrap();

                let terrain_resource: SUnk8080714b = TigerReadable::read_ds(table_data).unwrap();

                spawn_data_entity(
                    scene,
                    (
                        Icon(ICON_IMAGE_FILTER_HDR),
                        Label::from("Terrain Patches"),
                        TerrainPatches::load(renderer, terrain_resource.terrain)
                            .context("Failed to load terrain patches")?,
                        TfxFeatureRenderer::TerrainPatch,
                        resource_origin,
                    ),
                    parent_entity,
                );
            }
            0x80806aa3 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();
                let tag: TagHash = TigerReadable::read_ds(table_data).unwrap();
                if tag.is_none() {
                    continue;
                }

                let header: SUnk80806aa7 = package_manager().read_tag_struct(tag).unwrap();

                for (unk8, unk18, _unk28) in
                    multizip((header.unk8.iter(), header.unk18.iter(), header.unk28.iter()))
                {
                    if unk8.bounds != unk18.bb {
                        warn!(
                            "Bounds mismatch in Unk80806aa3: {:?} != {:?}",
                            unk8.bounds, unk18.bb
                        );
                    }

                    let transform = Transform::from_mat4(Mat4::from_cols_array(&unk8.transform));
                    spawn_data_entity(
                        scene,
                        (
                            Icon(ICON_WEATHER_PARTLY_CLOUDY),
                            Label::from("Sky Model"),
                            transform,
                            DynamicModelComponent::load(
                                renderer,
                                &transform,
                                unk8.unk60.entity_model,
                                vec![],
                                vec![],
                                TfxFeatureRenderer::SkyTransparent,
                            )?,
                            TfxFeatureRenderer::SkyTransparent,
                            resource_origin,
                        ),
                        parent_entity,
                    );
                }
            }
            0x808068d4 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset))
                    .unwrap();

                let d: SUnk808068d4 = TigerReadable::read_ds(table_data)?;

                if d.entity_model.is_some() {
                    spawn_data_entity(
                        scene,
                        (
                            Icon(ICON_WAVES),
                            Label::from("Water"),
                            transform,
                            DynamicModelComponent::load(
                                renderer,
                                &transform,
                                d.entity_model,
                                vec![],
                                vec![],
                                TfxFeatureRenderer::Water,
                            )?,
                            TfxFeatureRenderer::Water,
                            resource_origin,
                        ),
                        parent_entity,
                    );
                } else {
                    warn!(
                        "Water entity model is None (table {}, offset 0x{:X})",
                        table_hash, data.data_resource.offset
                    );
                }
            }
            0x80806a63 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();
                let tag: TagHash = table_data.read_le().unwrap();
                if !tag.is_some() {
                    continue;
                }

                let light_collection: SLightCollection =
                    package_manager().read_tag_struct(tag).unwrap();

                let light_collection_entity = spawn_data_entity(scene, (), parent_entity);
                let mut children = vec![];
                for (i, (light, transform, bounds)) in multizip((
                    light_collection.unk30.clone(),
                    light_collection.unk40.clone(),
                    light_collection.occlusion_bounds.bounds.iter(),
                ))
                .enumerate()
                {
                    children.push(
                        scene.spawn((
                            Icon(ICON_LIGHTBULB_ON),
                            Label::from(format!("Light {i}")),
                            Transform {
                                translation: transform.translation.xyz(),
                                rotation: transform.rotation,
                                ..Default::default()
                            },
                            LightRenderer::load(
                                renderer.gpu.clone(),
                                &mut renderer.data.lock().asset_manager,
                                &light,
                                format!("light {tag}+{i}"),
                            )
                            .context("Failed to load light")?,
                            light,
                            bounds.bb,
                            TfxFeatureRenderer::DeferredLights,
                            resource_origin,
                            Parent(light_collection_entity),
                        )),
                    );
                }

                scene.insert(
                    light_collection_entity,
                    (
                        light_collection,
                        Icon(ICON_LIGHTBULB_GROUP),
                        Label::from(format!("Light Collection {tag}")),
                        Children::from_slice(&children),
                    ),
                )?;
            }
            0x80806c5e => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();
                let tag: TagHash = table_data.read_le().unwrap();
                let light: SShadowingLight = package_manager().read_tag_struct(tag)?;

                let shadowmap = ShadowMapRenderer::new(
                    &renderer.gpu,
                    transform,
                    CameraProjection::perspective_bounded(
                        (light.half_fov * 2.).to_degrees(),
                        0.5,
                        light.far_plane,
                    ),
                )?;

                spawn_data_entity(
                    scene,
                    (
                        Icon(ICON_SPOTLIGHT_BEAM),
                        Label::from(format!("Shadowing Light {tag}")),
                        transform,
                        LightRenderer::load_shadowing(
                            renderer.gpu.clone(),
                            &mut renderer.data.lock().asset_manager,
                            &light,
                            format!("shadowing_light {tag}"),
                        )
                        .context("Failed to load shadowing light")?,
                        shadowmap,
                        light,
                        TfxFeatureRenderer::DeferredLights,
                        resource_origin,
                    ),
                    parent_entity,
                );
            }
            0x80806BC1 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();

                let atmos: SMapAtmosphere = TigerReadable::read_ds(table_data)?;
                spawn_data_entity(
                    scene,
                    (
                        Icon(ICON_WEATHER_FOG),
                        Label::from(format!(
                            "Atmosphere Configuration (table {}@0x{:X})",
                            table_hash, data.data_resource.offset
                        )),
                        MapAtmosphere::load(&renderer.gpu, atmos)
                            .context("Failed to load map atmosphere")?,
                        resource_origin,
                    ),
                    // parent_entity,
                    None,
                );
            }
            // Cubemap volume
            0x80806695 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset))
                    .unwrap();

                match SCubemapVolume::read_ds(table_data) {
                    Ok(cubemap_volume) => {
                        let voxel_diffuse = if cubemap_volume.voxel_ibl_texture.is_some() {
                            Some(
                                renderer
                                    .data
                                    .lock()
                                    .asset_manager
                                    .get_or_load_texture(cubemap_volume.voxel_ibl_texture),
                            )
                        } else {
                            None
                        };

                        spawn_data_entity(
                            scene,
                            (
                                Icon(ICON_SPHERE),
                                Label::from(format!(
                                    "Cubemap Volume '{}'",
                                    "<unknown>" // cubemap_volume
                                                //     .cubemap_name
                                                //     .to_string()
                                                //     .truncate_ellipsis(48)
                                )),
                                Transform {
                                    translation: data.translation.xyz(),
                                    rotation: transform.rotation,
                                    ..Default::default()
                                },
                                CubemapVolume {
                                    specular_ibl: renderer
                                        .data
                                        .lock()
                                        .asset_manager
                                        .get_or_load_texture(cubemap_volume.cubemap_texture),
                                    voxel_diffuse,
                                    extents: cubemap_volume.cubemap_extents.truncate(),
                                    // name: cubemap_volume.cubemap_name.to_string(),
                                    name: "<unknown>".to_string(),
                                },
                            ),
                            parent_entity,
                        );
                    }
                    Err(e) => error!("Failed to load cubemap volume: {e:?}"),
                }
            }
            0x808067b5 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();
                let tag: TagHash = table_data.read_le().unwrap();
                if tag.is_none() {
                    // cohae: Apparently the lens flare tag is optional?
                    continue;
                }

                let lens_flare: SLensFlare = package_manager().read_tag_struct(tag)?;

                spawn_data_entity(
                    scene,
                    (
                        Icon(ICON_FLARE),
                        Label::from("Lens Flare"),
                        transform,
                        lens_flare,
                        resource_origin,
                    ),
                    parent_entity,
                );
            }
            0x80808cb5 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();
                let tag: TagHash = table_data.read_le().unwrap();
                if !tag.is_some() {
                    continue;
                }

                let header: SUnk80808cb7 = package_manager().read_tag_struct(tag)?;

                for respawn_point in header.unk8.iter() {
                    spawn_data_entity(
                        scene,
                        (
                            Icon(ICON_ACCOUNT_CONVERT),
                            Label::from("Respawn point"),
                            Transform {
                                translation: respawn_point.translation.truncate(),
                                rotation: respawn_point.rotation,
                                ..Default::default()
                            },
                            respawn_point.clone(),
                            resource_origin,
                        ),
                        parent_entity,
                    );
                }
            }
            // Decorator
            0x80806cc3 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();
                let header_tag: TagHash = table_data.read_le().unwrap();
                let header: SDecorator = package_manager().read_tag_struct(header_tag)?;

                match DecoratorRenderer::load(renderer, header_tag, header) {
                    Ok(decorator_renderer) => {
                        spawn_data_entity(
                            scene,
                            (
                                Icon(ICON_TREE),
                                Label::from(format!("Decorator {header_tag}")),
                                decorator_renderer,
                            ),
                            parent_entity,
                        );
                    }
                    Err(e) => {
                        error!("Failed to load decorator {header_tag}: {e}");
                    }
                }
            }
            u => {
                if u != u32::MAX {
                    warn!("Unknown resource type {u:08X} in table {table_hash}");
                }
                let entity_hash = data.entity.hash32();
                if entity_hash.is_none() {
                    continue;
                }

                load_entity_into_scene(
                    entity_hash,
                    scene,
                    renderer,
                    resource_origin,
                    parent_entity,
                    transform,
                    if u != u32::MAX { Some(u) } else { None },
                    0,
                )
                .ok();
            }
        }
    }

    Ok(())
}

fn spawn_data_entity(
    scene: &mut Scene,
    components: impl DynamicBundle,
    parent: Option<Entity>,
) -> Entity {
    let child = scene.spawn(components);
    if let Some(parent) = parent {
        scene.set_parent(child, parent);
    }
    if let Ok(transform) = scene.get::<&Transform>(child).map(|t| *t.deref()) {
        scene
            .insert_one(child, OriginalTransform(transform))
            .unwrap();
    }

    child
}

fn get_entity_labels(entity: TagHash) -> Option<FxHashMap<u64, String>> {
    let data: Vec<u8> = package_manager().read_tag(entity).ok()?;
    let mut cur = Cursor::new(&data);

    let e: SEntityResource = TigerReadable::read_ds(&mut cur).ok()?;
    let mut world_id_list: Vec<Unk80809905> = vec![];
    if e.unk80.is_none() {
        return None;
    }

    for (i, b) in data.chunks_exact(4).enumerate() {
        let v: [u8; 4] = b.try_into().unwrap();
        let hash = u32::from_le_bytes(v);
        let offset = i as u64 * 4;

        if hash == 0x80809905 {
            cur.seek(SeekFrom::Start(offset - 8)).ok()?;
            let count: u64 = TigerReadable::read_ds(&mut cur).ok()?;
            cur.seek(SeekFrom::Start(offset + 8)).ok()?;
            for _ in 0..count {
                let e: Unk80809905 = TigerReadable::read_ds(&mut cur).ok()?;
                world_id_list.push(e);
            }
            // let list: TablePointer<Unk80809905> = TigerReadable::read_ds_endian(&mut cur, Endian::Little).ok()?;
            // world_id_list = list.take_data();
            break;
        }
    }

    // TODO(cohae): There's volumes and stuff without a world ID that still have a name
    world_id_list.retain(|w| w.world_id != u64::MAX);

    let mut name_hash_map: FxHashMap<FnvHash, String> = FxHashMap::default();

    let tablethingy: Unk8080906b = package_manager().read_tag_struct(e.unk80).ok()?;
    for v in tablethingy.unk0.into_iter() {
        if let Some(name_ptr) = v.unk0_name_pointer.as_ref() {
            name_hash_map.insert(
                fnv1(name_ptr.name.0 .0.as_bytes()),
                name_ptr.name.to_string(),
            );
        }
    }

    Some(
        world_id_list
            .into_iter()
            .filter_map(|w| Some((w.world_id, name_hash_map.get(&w.name_hash)?.clone())))
            .collect(),
    )
}

fn load_entity_into_scene(
    entity_hash: TagHash,
    scene: &mut Scene,
    renderer: &Renderer,
    resource_origin: ResourceOrigin,
    parent_entity: Option<Entity>,
    transform: Transform,
    u: Option<u32>,
    depth: usize,
) -> anyhow::Result<Entity> {
    // println!("{entity_hash} depth={depth}");
    // TODO(cohae): Shouldnt be possible, but happens anyways on certain maps like heaven/hell
    if depth > 8 {
        error!("Entity recursion depth exceeded for entity_hash={entity_hash}");
        return Err(anyhow::anyhow!("Entity recursion depth limit exceeded"));
    }

    if package_manager()
        .get_entry(entity_hash)
        .map_or(true, |v| Some(v.reference) != SEntity::ID)
    {
        return Ok(Entity::DANGLING);
    }

    let header = package_manager()
        .read_tag_struct::<SEntity>(entity_hash)
        .context("Failed to read SEntity")?;
    debug!("Loading entity {entity_hash}");
    let scene_entity = spawn_data_entity(
        scene,
        (
            Icon(ICON_CUBE),
            if let Some(u) = u {
                Label::from(format!("Unknown {u:08X}"))
            } else {
                Label::from("Generic Entity")
            },
            transform,
            resource_origin,
        ),
        parent_entity,
    );

    for e in &header.entity_resources {
        let entres = &e.unk0;
        match entres.unk10.resource_type {
            0x80806d8a => {
                let mut cur = Cursor::new(package_manager().read_tag(entres.taghash())?);
                cur.seek(SeekFrom::Start(entres.unk18.offset + 0x224))?;
                let model_hash: TagHash = TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                cur.seek(SeekFrom::Start(entres.unk18.offset + 0x3c0))?;
                let entity_material_map: Vec<Unk808072c5> =
                    TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                // cur.seek(SeekFrom::Start(entref.unk18.offset + 0x3f0))?;
                // let entity_material_map_pre: Vec<(u16, u16)> =
                //     TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                cur.seek(SeekFrom::Start(entres.unk18.offset + 0x400))?;
                let materials: Vec<TagHash> =
                    TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                scene.insert(
                    scene_entity,
                    (
                        DynamicModelComponent::load(
                            renderer,
                            &transform,
                            model_hash,
                            entity_material_map,
                            materials,
                            TfxFeatureRenderer::DynamicObjects,
                        )?,
                        TfxFeatureRenderer::DynamicObjects,
                    ),
                )?;
            }
            u => {
                debug!(
                    "\t- Unknown entity resource type {:08X}/{:08X} (table {})",
                    u.to_be(),
                    entres.unk10.resource_type.to_be(),
                    entres.taghash()
                )
            }
        }

        let mut loaded = FxHashSet::<WideHash>::default();
        for r in &entres.resource_table2 {
            // if matches!(r.unk14, 0 | 0xFFFFFFFF) {
            if r.unk14 == 0xFFFFFFFF {
                continue;
            }

            if loaded.contains(&r.unk0) {
                continue;
            }

            // SEntity::ID
            if r.unk0.is_some() {
                load_entity_into_scene(
                    r.unk0.hash32(),
                    scene,
                    renderer,
                    resource_origin,
                    Some(scene_entity),
                    // TODO(cohae): transform hierarchy
                    transform,
                    None,
                    depth + 1,
                )?;
                loaded.insert(r.unk0);
            }
        }
    }

    Ok(scene_entity)
}
const FNV1_BASE: u32 = 0x811c9dc5;
const FNV1_PRIME: u32 = 0x01000193;
fn fnv1(data: &[u8]) -> FnvHash {
    data.iter().fold(FNV1_BASE, |acc, b| {
        acc.wrapping_mul(FNV1_PRIME) ^ (*b as u32)
    })
}
