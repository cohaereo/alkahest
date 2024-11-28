use std::io::{Cursor, Read, Seek, SeekFrom};

use alkahest_data::{
    activity::{SActivity, SEntityResource, SUnk8080460c, Unk80808cef, Unk80808e89, Unk808092d8},
    common::ResourceHash,
    decorator::SDecorator,
    entity::{SEntity, Unk808072c5, Unk8080906b, Unk80809905},
    map::{
        SAudioClipCollection, SBubbleDefinition, SBubbleParent, SCubemapVolume,
        SDecalCollectionResource, SLensFlare, SLightCollection, SMapAtmosphere, SMapDataTable,
        SShadowingLight, SSlipSurfaceVolume, SStaticAmbientOcclusion, SUnk808068d4, SUnk80806aa7,
        SUnk80806ef4, SUnk8080714b, SUnk80808604, SUnk80808cb7, SUnk80809178, SUnk8080917b,
    },
    occlusion::Aabb,
    text::{StringContainer, StringContainerShared},
    tfx::TfxFeatureRenderer,
    Tag, WideHash,
};
use alkahest_pm::package_manager;
use anyhow::Context;
use bevy_ecs::{bundle::Bundle, entity::Entity, query::With};
use binrw::BinReaderExt;
use destiny_pkg::TagHash;
use ecolor::Color32;
use glam::{Mat4, Vec3, Vec4Swizzles};
use itertools::{multizip, Itertools};
use rustc_hash::{FxHashMap, FxHashSet};
use tiger_parse::{Endian, FnvHash, PackageManagerExt, TigerReadable};

use crate::{
    camera::CameraProjection,
    ecs::{
        audio::AmbientAudio,
        common::{Icon, Label, RenderCommonBundle, ResourceOrigin},
        hierarchy::{Children, Parent},
        map::{CubemapVolume, MapAtmosphere, MapStaticAO, NodeMetadata},
        render::{
            decorators::DecoratorRenderer,
            dynamic_geometry::DynamicModelComponent,
            havok::HavokShapeRenderer,
            light::{LightRenderer, LightShape, ShadowMapRenderer},
            static_geometry::{StaticInstance, StaticInstances, StaticModel, StaticModelSingle},
            terrain::TerrainPatches,
        },
        tags::{insert_tag, EntityTag, NodeFilter},
        transform::{OriginalTransform, Transform, TransformFlags},
        visibility::VisibilityBundle,
        Scene, SceneInfo,
    },
    icons::{
        ICON_ACCOUNT_CONVERT, ICON_CUBE, ICON_CUBE_OUTLINE, ICON_FLARE, ICON_FOLDER,
        ICON_IMAGE_FILTER_HDR, ICON_LABEL, ICON_LIGHTBULB_GROUP, ICON_SHAPE, ICON_SPEAKER,
        ICON_SPHERE, ICON_SPOTLIGHT_BEAM, ICON_STICKER, ICON_TREE, ICON_WAVES, ICON_WEATHER_FOG,
        ICON_WEATHER_PARTLY_CLOUDY,
    },
    renderer::{Renderer, RendererShared},
    util::{
        scene::{EntityWorldMutExt, SceneExt},
        text::StringExt,
    },
};

pub async fn load_map(
    renderer: RendererShared,
    map_hash: TagHash,
    activity_hash: Option<TagHash>,
    stringmap: StringContainerShared,
    load_ambient_activity: bool,
) -> anyhow::Result<Scene> {
    let bubble_parent = package_manager()
        .read_tag_struct::<SBubbleParent>(map_hash)
        .context("Failed to read SBubbleParent")?;

    let mut scene = Scene::new_with_info(activity_hash, map_hash);
    let bubble_definition = if bubble_parent.child_map.is_some() {
        package_manager()
            .read_tag_struct::<SBubbleDefinition>(bubble_parent.child_map)
            .context("Failed to read bubble definition")?
    } else {
        warn!("Map {map_hash} is missing a bubble definition!");
        return Ok(scene);
    };
    // let Ok(bubble_definition) = package_manager().read_tag::<SBubbleDefinition>(bubble_parent.child_map) else {
    //     warn!("Failed to load bubble definition for map {}", map_hash);
    // }

    let mut data_tables = FxHashMap::<TagHash, Entity>::default();
    for map_container in &bubble_definition.map_resources {
        let parent_entity =
            scene.spawn((Label::from(format!("Map Container {}", map_container.1)),));
        for table in &map_container.data_tables {
            data_tables.insert(*table, parent_entity.id());
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
            &stringmap,
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

        if load_ambient_activity && activity.ambient_activity.is_some() {
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
            scene
                .spawn((Label::from(format!(
                    "Activity Phase 0x{:08X}",
                    phase_name2.0
                )),))
                .id()
        });

        for resource in &e.unk18.entity_resources {
            if resource.entity_resource.is_some() {
                let data = package_manager().read_tag(resource.entity_resource)?;
                let mut cur = Cursor::new(&data);
                let res: SEntityResource = TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                let mut data_tables: FxHashMap<TagHash, Option<Entity>> = FxHashMap::default();
                match res.unk18.resource_type {
                    0x808092d8 => {
                        cur.seek(SeekFrom::Start(res.unk18.offset))?;
                        let tag: Unk808092d8 =
                            TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                        if tag.unk84.is_some() {
                            let entity = scene.spawn((
                                Label::from(format!("Activity Datatable {}", tag.unk84)),
                                Transform::new(tag.translation.truncate(), tag.rotation, Vec3::ONE),
                            ));

                            data_tables.insert(tag.unk84, Some(entity.id()));
                        }
                    }
                    0x80808cef => {
                        cur.seek(SeekFrom::Start(res.unk18.offset))?;
                        let tag: Unk80808cef =
                            TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;
                        if tag.unk58.is_some() {
                            data_tables.insert(tag.unk58, None);
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
                        && !data_tables.contains_key(&hash)
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

                for (table_hash, table_entity) in data_tables {
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
                        table_entity.or(Some(parent_entity)),
                        &stringmap,
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
                        &stringmap,
                    )
                    .context("Failed to load AB datatable")?;
                }

                if origin != ResourceOrigin::Ambient {
                    for r in &res.resource_table2 {
                        if r.unk14 != 0xFFFFFFFF && r.unk0.is_some() {
                            let transform = if res.unk18.resource_type == 0x8080460C {
                                cur.seek(SeekFrom::Start(res.unk18.offset))?;
                                let tag: SUnk8080460c =
                                    TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;
                                Transform::new(tag.translation.truncate(), tag.rotation, Vec3::ONE)
                            } else {
                                Transform::default()
                            };

                            // SEntity::ID
                            load_entity_into_scene(
                                r.unk0.hash32(),
                                &mut scene,
                                &renderer,
                                origin,
                                None,
                                transform,
                                None,
                                0,
                                None,
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
    for e in scene.iter_entities() {
        let mut tag_list = vec![];
        if let Some(origin) = e.get::<ResourceOrigin>().cloned() {
            match origin {
                ResourceOrigin::Map => {}
                ResourceOrigin::Activity => tag_list.push(EntityTag::Activity),
                ResourceOrigin::ActivityBruteforce => tag_list.push(EntityTag::Activity),
                ResourceOrigin::Ambient => tag_list.push(EntityTag::Ambient),
            }
        }

        // TODO(cohae): Havok tags

        tags.push((e.id(), tag_list));
    }

    for (e, tags) in tags {
        for tag in tags {
            insert_tag(&mut scene, e, tag);
        }
    }

    let mut new_entity_names: Vec<(Entity, String)> = vec![];
    for (entity, mut meta) in scene
        .query::<(Entity, &mut NodeMetadata)>()
        .iter_mut(&mut scene)
    {
        if meta.world_id != u64::MAX {
            if let Some(name) = entity_worldid_name_map.get(&meta.world_id) {
                new_entity_names.push((entity, name.clone()));
                meta.name = Some(name.clone());
            }
        }
    }

    for (entity, name) in new_entity_names {
        scene.entity_mut(entity).insert_one(Label::from(name));
    }

    let mut entity_ogtransforms: Vec<(Entity, OriginalTransform)> = vec![];
    for (entity, transform) in scene.query::<(Entity, &Transform)>().iter(&scene) {
        entity_ogtransforms.push((entity, OriginalTransform(*transform)));
    }

    for (entity, transform) in entity_ogtransforms {
        scene.entity_mut(entity).insert_one(transform);
    }

    let mut to_update = vec![];
    for entity in scene
        .query_filtered::<Entity, With<TerrainPatches>>()
        .iter(&scene)
    {
        to_update.push(entity);
    }
    // Vertex AO: refresh terrain constants
    if let Some(map_ao) = scene.get_resource::<MapStaticAO>() {
        for e in to_update {
            let patches = scene.entity(e).get::<TerrainPatches>().unwrap();
            patches.update_constants(map_ao);
        }
    }

    Ok(scene)
}

#[allow(clippy::too_many_arguments)]
fn load_datatable_into_scene<R: Read + Seek>(
    table: &SMapDataTable,
    table_hash: TagHash,
    table_data: &mut R,
    scene: &mut Scene,
    renderer: &Renderer,
    resource_origin: ResourceOrigin,
    parent_entity: Option<Entity>,
    stringmap: &StringContainer,
) -> anyhow::Result<()> {
    for data in table.data_entries.iter() {
        let transform = Transform {
            translation: Vec3::new(data.translation.x, data.translation.y, data.translation.z),
            rotation: data.rotation,
            scale: Vec3::splat(data.translation.w),
            ..Default::default()
        };

        let metadata = NodeMetadata {
            entity_tag: data.entity.hash32(),
            world_id: data.world_id,
            source_table: table_hash,
            source_table_resource_offset: data.data_resource.offset,
            resource_type: data.data_resource.resource_type,
            name: None,
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

                    let bounds = if ((s.instance_start + s.instance_count) as usize)
                        < preheader.instances.occlusion_bounds.bounds.len()
                    {
                        &preheader.instances.occlusion_bounds.bounds[s.instance_start as usize
                            ..(s.instance_start + s.instance_count) as usize]
                    } else {
                        &[]
                    };

                    // Load model as a single entity if it only has one instance
                    if transforms.len() == 1 {
                        let transform = Transform {
                            translation: transforms[0].translation,
                            rotation: transforms[0].rotation,
                            scale: Vec3::splat(transforms[0].scale.x),
                            flags: TransformFlags::empty(),
                        };

                        let parent = spawn_data_entity(scene, (metadata.clone(),), parent_entity);
                        scene.entity_mut(parent).insert((
                            Icon::Unicode(ICON_SHAPE),
                            Label::from(format!("Static Model {mesh_tag}")),
                            transform,
                            StaticModelSingle::new(renderer.gpu.clone(), model)?,
                            TfxFeatureRenderer::StaticObjects,
                            resource_origin,
                            NodeFilter::Static,
                        ));

                        if let Some(bounds) = bounds.first() {
                            scene
                                .entity_mut(parent)
                                .insert_one(bounds.bb.untransform(transform.local_to_world()));
                        }
                    } else {
                        let parent = spawn_data_entity(scene, (metadata.clone(),), parent_entity);
                        let mut instances = vec![];

                        for (i, transform) in transforms.iter().enumerate() {
                            let transform = Transform {
                                translation: transform.translation,
                                rotation: transform.rotation,
                                scale: Vec3::splat(transform.scale.x),
                                flags: TransformFlags::empty(),
                            };

                            let mut entity = scene.spawn((
                                Icon::Unicode(ICON_CUBE_OUTLINE),
                                Label::from("Static Instance"),
                                transform,
                                StaticInstance,
                                Parent(parent),
                                NodeFilter::Static,
                                RenderCommonBundle::default(),
                            ));

                            if let Some(bounds) = bounds.get(i) {
                                entity
                                    .insert_one(bounds.bb.untransform(transform.local_to_world()));
                            }

                            instances.push(entity.id());
                        }
                        scene.entity_mut(parent).insert((
                            Icon::Unicode(ICON_SHAPE),
                            Label::from(format!("Static Instances {mesh_tag}")),
                            StaticInstances::new(renderer.gpu.clone(), model, instances.len())?,
                            Children::from_slice(&instances),
                            TfxFeatureRenderer::StaticObjects,
                            resource_origin,
                            NodeFilter::Static,
                            RenderCommonBundle::default(),
                        ));
                    }
                }
            }
            // D2Class_7D6C8080 (terrain)
            0x80806c7d => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset))
                    .unwrap();

                let terrain_resource: SUnk8080714b = TigerReadable::read_ds(table_data).unwrap();
                let terrain_renderer = TerrainPatches::load_from_tag(
                    renderer,
                    terrain_resource.terrain,
                    terrain_resource.identifier,
                )
                .context("Failed to load terrain patches")?;

                spawn_data_entity(
                    scene,
                    (
                        Icon::Unicode(ICON_IMAGE_FILTER_HDR),
                        Label::from("Terrain Patches"),
                        terrain_renderer.terrain.bounds,
                        terrain_renderer,
                        TfxFeatureRenderer::TerrainPatch,
                        resource_origin,
                        metadata.clone(),
                    ),
                    parent_entity,
                );
            }
            // Decal collection
            0x80806955 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();
                let tag: TagHash = table_data.read_le().unwrap();
                if !tag.is_some() {
                    continue;
                }

                let header: SDecalCollectionResource =
                    package_manager().read_tag_struct(tag).unwrap();

                let decal_collection_entity =
                    spawn_data_entity(scene, (metadata.clone(),), parent_entity);
                let mut children = vec![];
                for inst in &header.instance_ranges {
                    for i in inst.start..(inst.start + inst.count) {
                        let transform = header.transforms[i as usize];
                        // let bounds = &header.occlusion_bounds.bounds[i as usize];
                        children.push(spawn_data_entity(
                            scene,
                            (
                                Transform {
                                    translation: Vec3::new(transform.x, transform.y, transform.z),
                                    ..Default::default()
                                },
                                Icon::Colored(ICON_STICKER, Color32::from_rgb(24, 201, 186)),
                                Label::from(format!("Decal (material={})", inst.material)),
                                resource_origin,
                                NodeFilter::Decal,
                                metadata.clone(),
                            ),
                            Some(decal_collection_entity),
                        ));
                    }
                }

                scene.entity_mut(decal_collection_entity).insert((
                    Icon::Unicode(ICON_FOLDER),
                    Label::from(format!("Decal Collection {tag}")),
                    Children::from_slice(&children),
                    resource_origin,
                    // RenderCommonBundle::default(),
                ));
            }
            // (ambient) sound source
            0x8080666f => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();

                let tag: WideHash = TigerReadable::read_ds(table_data).unwrap();
                let entity = spawn_data_entity(
                    scene,
                    (
                        NodeFilter::Sound,
                        Icon::Colored(ICON_SPEAKER, Color32::GREEN),
                        Label::from(format!("Ambient Audio {}", tag.hash32())),
                        transform,
                        resource_origin,
                        metadata.clone(),
                    ),
                    parent_entity,
                );
                if tag.hash32().is_none() {
                    warn!(
                        "Sound source tag is None ({tag}, table {}, offset 0x{:X})",
                        table_hash, data.data_resource.offset
                    );
                } else {
                    match package_manager().read_tag_struct::<SAudioClipCollection>(tag) {
                        Ok(header) => {
                            scene
                                .entity_mut(entity)
                                .insert_one(AmbientAudio::new(header));
                        }
                        Err(e) => {
                            error!(error=?e, tag=%tag, "Failed to load ambient audio");
                        }
                    }
                }
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

                    // let thingy = [
                    //     TagHash(0x80C9E4FD),
                    //     TagHash(0x80B12BA6),
                    //     TagHash(0x80DED603),
                    //     TagHash(0x80DED488),
                    //     TagHash(0x80DEA0AD),
                    // ];
                    // if thingy.contains(&unk8.unk60.entity_model) {
                    // println!("{}", unk8.unk60.entity_model);
                    // println!("{unk8:#X?}");
                    // }

                    if unk8.unk70 == 5 {
                        continue;
                    }

                    let model = DynamicModelComponent::load(
                        renderer,
                        &transform,
                        unk8.unk60.entity_model,
                        vec![],
                        vec![],
                        TfxFeatureRenderer::SkyTransparent,
                    )?;
                    let transform = Transform::from_mat4(Mat4::from_cols_array(&unk8.transform));
                    spawn_data_entity(
                        scene,
                        (
                            NodeFilter::SkyObject,
                            Icon::Colored(ICON_WEATHER_PARTLY_CLOUDY, Color32::LIGHT_BLUE),
                            Label::from(format!("Sky Model {}", unk8.unk60.entity_model)),
                            transform,
                            model.model.occlusion_bounds(),
                            model,
                            TfxFeatureRenderer::SkyTransparent,
                            resource_origin,
                            metadata.clone(),
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

                if d.entity_model.is_none() {
                    warn!(
                        "Water entity model is None (table {}, offset 0x{:X})",
                        table_hash, data.data_resource.offset
                    );
                    continue;
                }

                let model = DynamicModelComponent::load(
                    renderer,
                    &transform,
                    d.entity_model,
                    vec![],
                    vec![],
                    TfxFeatureRenderer::Water,
                )?;
                if d.entity_model.is_some() {
                    spawn_data_entity(
                        scene,
                        (
                            Icon::Unicode(ICON_WAVES),
                            Label::from("Water"),
                            transform,
                            model.model.occlusion_bounds(),
                            model,
                            TfxFeatureRenderer::Water,
                            resource_origin,
                            metadata.clone(),
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
            0x80806a40 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();
                let tag: TagHash = table_data.read_le().unwrap();
                if tag.is_none() {
                    continue;
                }

                let static_ao =
                    match package_manager().read_tag_struct::<SStaticAmbientOcclusion>(tag) {
                        Ok(static_ao) => static_ao,
                        Err(e) => {
                            error!(error=?e, tag=%tag, "Failed to load static AO");
                            continue;
                        }
                    };

                scene.insert_resource(
                    MapStaticAO::from_tag(&renderer.gpu, &static_ao)
                        .context("Failed to load static AO")?,
                );
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

                let light_collection_entity =
                    spawn_data_entity(scene, (metadata.clone(),), parent_entity);
                let mut children = vec![];
                for (i, (light, transform, bounds)) in multizip((
                    light_collection.unk30.clone(),
                    light_collection.unk40.clone(),
                    light_collection.occlusion_bounds.bounds.iter(),
                ))
                .enumerate()
                {
                    let shape = LightShape::from_volume_matrix(light.light_to_world);
                    let transform = Transform {
                        translation: transform.translation.xyz(),
                        rotation: transform.rotation,
                        ..Default::default()
                    };
                    children.push(
                        scene
                            .spawn((
                                NodeFilter::Light,
                                Icon::Colored(shape.icon(), Color32::YELLOW),
                                Label::from(format!("{} Light {tag}[{i}]", shape.name())),
                                transform,
                                LightRenderer::load(
                                    renderer.gpu.clone(),
                                    &mut renderer.data.lock().asset_manager,
                                    &light,
                                    format!("light {tag}+{i}"),
                                )
                                .context("Failed to load light")?,
                                light,
                                bounds.bb.untransform(transform.local_to_world()),
                                TfxFeatureRenderer::DeferredLights,
                                resource_origin,
                                Parent(light_collection_entity),
                                RenderCommonBundle::default(),
                            ))
                            .id(),
                    );
                }

                scene.entity_mut(light_collection_entity).insert((
                    light_collection,
                    Icon::Unicode(ICON_LIGHTBULB_GROUP),
                    Label::from(format!("Light Collection {tag}")),
                    Children::from_slice(&children),
                ));
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
                    renderer.settings.shadow_quality.resolution(),
                )?;

                let bb = Aabb::from_projection_matrix(light.light_to_world);

                spawn_data_entity(
                    scene,
                    (
                        NodeFilter::Light,
                        Icon::Colored(ICON_SPOTLIGHT_BEAM, Color32::YELLOW),
                        Label::from(format!("Shadowing Spotlight {tag}")),
                        transform,
                        LightRenderer::load_shadowing(
                            renderer.gpu.clone(),
                            &mut renderer.data.lock().asset_manager,
                            &light,
                            format!("shadowing_light {tag}"),
                        )
                        .context("Failed to load shadowing light")?,
                        shadowmap,
                        bb,
                        light,
                        TfxFeatureRenderer::DeferredLights,
                        resource_origin,
                        metadata.clone(),
                        RenderCommonBundle::default(),
                    ),
                    parent_entity,
                );
            }
            0x80806BC1 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();

                let atmos: SMapAtmosphere = TigerReadable::read_ds(table_data)?;
                scene.insert_resource(
                    MapAtmosphere::load(&renderer.gpu, atmos)
                        .context("Failed to load map atmosphere")?,
                );

                // Load as entity for ease of debugging
                scene.spawn((
                    Icon::Unicode(ICON_WEATHER_FOG),
                    Label::from(format!(
                        "Atmosphere Configuration (table {}@0x{:X})",
                        table_hash, data.data_resource.offset
                    )),
                    resource_origin,
                    metadata.clone(),
                ));
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
                                NodeFilter::Cubemap,
                                Icon::Unicode(ICON_SPHERE),
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
                                metadata.clone(),
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
                        NodeFilter::Light,
                        Icon::Unicode(ICON_FLARE),
                        Label::from("Lens Flare"),
                        transform,
                        lens_flare,
                        resource_origin,
                        metadata.clone(),
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
                            NodeFilter::RespawnPoint,
                            Icon::Colored(ICON_ACCOUNT_CONVERT, Color32::RED),
                            Label::from(format!("Respawn point 0x{:X}", respawn_point.unk20)),
                            Transform {
                                translation: respawn_point.translation.truncate(),
                                rotation: respawn_point.rotation,
                                ..Default::default()
                            },
                            respawn_point.clone(),
                            resource_origin,
                            metadata.clone(),
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
                                NodeFilter::Decorator,
                                Icon::Colored(ICON_TREE, Color32::LIGHT_GREEN),
                                Label::from(format!("Decorator {header_tag}")),
                                decorator_renderer,
                                metadata.clone(),
                            ),
                            parent_entity,
                        );
                    }
                    Err(e) => {
                        error!("Failed to load decorator {header_tag}: {e}");
                    }
                }
            }
            0x80809178 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset))
                    .unwrap();

                let d: SUnk80809178 = TigerReadable::read_ds(table_data)?;
                let name = stringmap.get(d.area_name);

                let (havok_debugshape, new_transform) =
                    if let Ok(havok_data) = package_manager().read_tag(d.unk0.havok_file) {
                        let mut cur = Cursor::new(&havok_data);
                        match destiny_havok::shape_collection::read_shape_collection(&mut cur) {
                            Ok(o) => {
                                if (d.unk0.shape_index as usize) < o.len() {
                                    let mut shape = o[d.unk0.shape_index as usize].clone();

                                    let center = shape.center();
                                    shape.apply_transform(Mat4::from_translation(-center));

                                    let new_transform = Transform::from_mat4(
                                        transform.local_to_world() * Mat4::from_translation(center),
                                    );

                                    (
                                        HavokShapeRenderer::new(renderer.gpu.clone(), &shape).ok(),
                                        Some(new_transform),
                                    )
                                } else {
                                    (None, None)
                                }
                            }
                            Err(e) => {
                                error!("Failed to read shapes: {e}");
                                (None, None)
                            }
                        }
                    } else {
                        (None, None)
                    };

                if let Some(havok_debugshape) = havok_debugshape {
                    spawn_data_entity(
                        scene,
                        (
                            new_transform.unwrap_or(transform),
                            NodeFilter::NamedArea,
                            Icon::Colored(ICON_LABEL, Color32::GREEN),
                            Label::from(format!("Named Area '{name}'")),
                            havok_debugshape,
                            metadata.clone(),
                        ),
                        parent_entity,
                    );
                }
            }
            // 0x80806abb => {
            //     table_data
            //         .seek(SeekFrom::Start(data.data_resource.offset + 16))
            //         .unwrap();
            //
            //     let (shape_ptr, shape_index): (TagHash, u32) = TigerReadable::read_ds(table_data)?;
            //
            //     println!("Shape: {}: {}", shape_ptr, shape_index);
            //
            //     let shapelist: SUnk80806abd = package_manager().read_tag_struct(shape_ptr)?;
            //
            //     let (havok_debugshape, new_transform) =
            //         if let Ok(havok_data) = package_manager().read_tag(shapelist.havok_file) {
            //             let mut cur = Cursor::new(&havok_data);
            //             match destiny_havok::shape_collection::read_shape_collection(&mut cur) {
            //                 Ok(o) => {
            //                     if (shape_index as usize) < o.len() {
            //                         let mut shape = o[shape_index as usize].clone();
            //
            //                         let center = shape.center();
            //                         shape.apply_transform(Mat4::from_translation(-center));
            //
            //                         let new_transform = Transform::from_mat4(
            //                             transform.to_mat4() * Mat4::from_translation(center),
            //                         );
            //
            //                         (
            //                             CustomDebugShape::from_havok_shape(&dcs, &shape).ok(),
            //                             Some(new_transform),
            //                         )
            //                     } else {
            //                         (None, None)
            //                     }
            //                 }
            //                 Err(e) => {
            //                     error!("Failed to read shapes: {e}");
            //                     (None, None)
            //                 }
            //             }
            //         } else {
            //             (None, None)
            //         };
            //
            //     ents.push(scene.spawn((
            //         new_transform.unwrap_or(transform),
            //         ResourcePoint {
            //             resource: MapResource::Unk80806abb(
            //                 shape_ptr,
            //                 shape_index,
            //                 havok_debugshape,
            //             ),
            //             has_havok_data: true,
            //             ..base_rp
            //         },
            //         EntityWorldId(data.world_id),
            //     )));
            //     spawn_data_entity(
            //         scene,
            //         (
            //             NodeFilter::NamedArea,
            //             Icon::Colored(ICON_TREE, Color32::LIGHT_GREEN),
            //             Label::from(format!("Named Area '{name}'")),
            //             havok_debugshape,
            //             metadata.clone(),
            //         ),
            //         parent_entity,
            //     );
            // }
            0x8080917b => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset))
                    .unwrap();

                let d: SUnk8080917b = TigerReadable::read_ds(table_data)?;

                let havok_debugshape =
                    if let Ok(havok_data) = package_manager().read_tag(d.unk0.havok_file) {
                        let mut cur = Cursor::new(&havok_data);
                        match destiny_havok::shape_collection::read_shape_collection(&mut cur) {
                            Ok(o) => {
                                if (d.unk0.shape_index as usize) < o.len() {
                                    HavokShapeRenderer::new(
                                        renderer.gpu.clone(),
                                        &o[d.unk0.shape_index as usize],
                                    )
                                    .ok()
                                } else {
                                    None
                                }
                            }
                            Err(e) => {
                                error!("Failed to read shapes: {e}");
                                None
                            }
                        }
                    } else {
                        None
                    };

                let filter = match d.kind {
                    0 => NodeFilter::InstakillBarrier,
                    1 => NodeFilter::TurnbackBarrier,
                    _ => {
                        error!("Unknown kill barrier type {}", d.kind);
                        NodeFilter::InstakillBarrier
                    }
                };

                if let Some(havok_debugshape) = havok_debugshape {
                    spawn_data_entity(
                        scene,
                        (
                            transform,
                            filter,
                            Icon::Colored(filter.icon(), filter.color().into()),
                            Label::from(filter.to_string().split_pascalcase()),
                            havok_debugshape,
                            metadata.clone(),
                        ),
                        parent_entity,
                    );
                }
            }
            0x80808604 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset))
                    .unwrap();

                let d: SUnk80808604 = TigerReadable::read_ds(table_data)?;

                let (havok_debugshape, new_transform) =
                    if let Ok(havok_data) = package_manager().read_tag(d.unk10.havok_file) {
                        let mut cur = Cursor::new(&havok_data);
                        match destiny_havok::shape_collection::read_shape_collection(&mut cur) {
                            Ok(shapes) => {
                                let t = &d.unk10.unk8[d.index as usize];
                                if t.shape_index as usize >= shapes.len() {
                                    error!(
                                        "Shape index out of bounds for Unk80808604 (table {}, {} \
                                         shapes, index {})",
                                        table_hash,
                                        shapes.len(),
                                        t.shape_index
                                    );
                                    continue;
                                }

                                let transform = Transform {
                                    translation: t.translation.truncate(),
                                    rotation: t.rotation,
                                    ..Default::default()
                                };

                                let mut shape = shapes[t.shape_index as usize].clone();
                                shape.apply_transform(transform.local_to_world());

                                // Re-center the shape
                                let center = shape.center();
                                shape.apply_transform(Mat4::from_translation(-center));

                                let new_transform = Transform {
                                    translation: center,
                                    ..Default::default()
                                };

                                (
                                    HavokShapeRenderer::new(renderer.gpu.clone(), &shape).ok(),
                                    Some(new_transform),
                                )
                            }
                            Err(e) => {
                                error!("Failed to read shapes: {e}");
                                (None, None)
                            }
                        }
                    } else {
                        (None, None)
                    };

                if let Some(havok_debugshape) = havok_debugshape {
                    let filter = NodeFilter::PlayerContainmentVolume;
                    spawn_data_entity(
                        scene,
                        (
                            new_transform.unwrap_or(transform),
                            filter,
                            Icon::Colored(filter.icon(), filter.color().into()),
                            Label::from("Player Containment Volume"),
                            havok_debugshape,
                            metadata.clone(),
                        ),
                        parent_entity,
                    );
                }
            }
            // 0x80808246 => {
            //     table_data
            //         .seek(SeekFrom::Start(data.data_resource.offset))
            //         .unwrap();
            //
            //     let d: SUnk80808246 = TigerReadable::read_ds(table_data)?;
            //
            //     match package_manager().read_tag(d.unk10.havok_file) {
            //         Ok(havok_data) => {
            //             let mut cur = Cursor::new(&havok_data);
            //             match destiny_havok::shape_collection::read_shape_collection(&mut cur) {
            //                 Ok(shapes) => {
            //                     for t in &d.unk10.unk10 {
            //                         if t.shape_index as usize >= shapes.len() {
            //                             error!(
            //                                 "Shape index out of bounds for Unk80808246 (table {}, \
            //                                  {} shapes, index {})",
            //                                 table_hash,
            //                                 shapes.len(),
            //                                 t.shape_index
            //                             );
            //                             continue;
            //                         }
            //
            //                         let transform = Transform {
            //                             translation: t.translation.truncate(),
            //                             rotation: t.rotation,
            //                             ..Default::default()
            //                         };
            //                         //
            //                         // ents.push(
            //                         //     scene.spawn((
            //                         //         transform,
            //                         //         ResourcePoint {
            //                         //             resource: MapResource::Unk80808246(
            //                         //                 d.unk10.havok_file,
            //                         //                 t.shape_index,
            //                         //                 CustomDebugShape::from_havok_shape(
            //                         //                     &dcs,
            //                         //                     &shapes[t.shape_index as usize],
            //                         //                 )
            //                         //                 .ok(),
            //                         //             ),
            //                         //             has_havok_data: true,
            //                         //             entity_cbuffer: ConstantBufferCached::create_empty(
            //                         //                 dcs.clone(),
            //                         //             )?,
            //                         //             ..base_rp
            //                         //         },
            //                         //         EntityWorldId(data.world_id),
            //                         //     )),
            //                         // );
            //                     }
            //
            //                     // let new_transform = Transform {
            //                     //     translation: center,
            //                     //     ..Default::default()
            //                     // };
            //
            //                     // (
            //                     //     CustomDebugShape::from_havok_shape(&dcs, &final_shape).ok(),
            //                     //     Some(new_transform),
            //                     // )
            //                 }
            //                 Err(e) => {
            //                     error!("Failed to read shapes: {e}");
            //                 }
            //             }
            //         }
            //         Err(e) => {
            //             error!("Failed to read shapes: {e}");
            //         }
            //     };
            // }
            // 0x80806ac2 => {
            //     table_data
            //         .seek(SeekFrom::Start(data.data_resource.offset))
            //         .unwrap();
            //
            //     let d: SUnk80806ac2 = TigerReadable::read_ds(table_data)?;
            //
            //     match package_manager().read_tag(d.unk10.havok_file) {
            //         Ok(havok_data) => {
            //             let mut cur = Cursor::new(&havok_data);
            //             match destiny_havok::shape_collection::read_shape_collection(&mut cur) {
            //                 Ok(shapes) => {
            //                     if let Some(t) = d.unk10.unk10.get(d.array_index as usize) {
            //                         if t.shape_index as usize >= shapes.len() {
            //                             error!(
            //                                 "Shape index out of bounds for Unk80808246 (table {}, \
            //                                  {} shapes, index {})",
            //                                 table_hash,
            //                                 shapes.len(),
            //                                 t.shape_index
            //                             );
            //
            //                             continue;
            //                         }
            //
            //                         let transform = Transform {
            //                             translation: t.translation.truncate(),
            //                             rotation: t.rotation,
            //                             ..Default::default()
            //                         };
            //
            //                         ents.push(
            //                             scene.spawn((
            //                                 transform,
            //                                 ResourcePoint {
            //                                     resource: MapResource::Unk80806ac2(
            //                                         d.unk10.havok_file,
            //                                         t.shape_index,
            //                                         CustomDebugShape::from_havok_shape(
            //                                             &dcs,
            //                                             &shapes[t.shape_index as usize],
            //                                         )
            //                                         .ok(),
            //                                     ),
            //                                     has_havok_data: true,
            //                                     entity_cbuffer: ConstantBufferCached::create_empty(
            //                                         dcs.clone(),
            //                                     )?,
            //                                     ..base_rp
            //                                 },
            //                                 EntityWorldId(data.world_id),
            //                             )),
            //                         );
            //                     }
            //                 }
            //                 Err(e) => {
            //                     error!("Failed to read shapes: {e}");
            //                 }
            //             }
            //         }
            //         Err(e) => {
            //             error!("Failed to read shapes: {e}");
            //         }
            //     };
            // }
            0x80809121 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset))
                    .unwrap();

                let d: SSlipSurfaceVolume = TigerReadable::read_ds(table_data)?;

                let (havok_debugshape, _new_transform) =
                    if let Ok(havok_data) = package_manager().read_tag(d.havok_file) {
                        let mut cur = Cursor::new(&havok_data);
                        match destiny_havok::shape_collection::read_shape_collection(&mut cur) {
                            Ok(o) => {
                                if (d.shape_index as usize) < o.len() {
                                    let mut shape = o[d.shape_index as usize].clone();

                                    let center = shape.center();
                                    shape.apply_transform(Mat4::from_translation(-center));

                                    let new_transform = Transform::from_mat4(
                                        transform.local_to_world() * Mat4::from_translation(center),
                                    );

                                    (
                                        HavokShapeRenderer::new(renderer.gpu.clone(), &shape).ok(),
                                        Some(new_transform),
                                    )
                                } else {
                                    (None, None)
                                }
                            }
                            Err(e) => {
                                error!("Failed to read shapes: {e}");
                                (None, None)
                            }
                        }
                    } else {
                        (None, None)
                    };

                if let Some(havok_debugshape) = havok_debugshape {
                    let filter = NodeFilter::SlipSurfaceVolume;
                    spawn_data_entity(
                        scene,
                        (
                            transform,
                            filter,
                            Icon::Colored(filter.icon(), filter.color().into()),
                            Label::from("Slip Surface Volume"),
                            havok_debugshape,
                            metadata.clone(),
                        ),
                        parent_entity,
                    );
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
                    Some(metadata),
                )
                .ok();
            }
        }
    }

    Ok(())
}

fn spawn_data_entity(scene: &mut Scene, components: impl Bundle, parent: Option<Entity>) -> Entity {
    let mut child = scene.spawn(components);
    child.insert(VisibilityBundle::default());

    let child_id = child.id();

    if let Some(parent) = parent {
        scene.set_parent(child_id, parent);
    }

    child_id
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

#[allow(clippy::too_many_arguments)]
fn load_entity_into_scene(
    entity_hash: TagHash,
    scene: &mut Scene,
    renderer: &Renderer,
    resource_origin: ResourceOrigin,
    parent_entity: Option<Entity>,
    transform: Transform,
    u: Option<u32>,
    depth: usize,
    metadata: Option<NodeMetadata>,
) -> anyhow::Result<Entity> {
    // TODO(cohae): Shouldnt be possible, but happens anyways on certain maps like heaven/hell
    if depth > 8 {
        error!("Entity recursion depth exceeded for entity_hash={entity_hash}");
        return Err(anyhow::anyhow!("Entity recursion depth limit exceeded"));
    }

    if package_manager()
        .get_entry(entity_hash)
        .map_or(true, |v| Some(v.reference) != SEntity::ID)
    {
        return Ok(Entity::PLACEHOLDER);
    }

    let header = package_manager()
        .read_tag_struct::<SEntity>(entity_hash)
        .context("Failed to read SEntity")?;
    let scene_entity = spawn_data_entity(
        scene,
        (
            Icon::Unicode(ICON_CUBE),
            if let Some(u) = u {
                Label::from(format!("Unknown {u:08X}"))
            } else {
                Label::from(format!("Entity {entity_hash}"))
            },
            if u.is_some() {
                NodeFilter::Unknown
            } else {
                NodeFilter::Entity
            },
            transform,
            resource_origin,
        ),
        parent_entity,
    );
    if let Some(metadata) = metadata {
        scene.entity_mut(scene_entity).insert_one(metadata);
    }

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

                cur.seek(SeekFrom::Start(entres.unk18.offset + 0x400))?;
                let materials: Vec<TagHash> =
                    TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                let model = DynamicModelComponent::load(
                    renderer,
                    &transform,
                    model_hash,
                    entity_material_map,
                    materials,
                    TfxFeatureRenderer::DynamicObjects,
                )?;
                scene.entity_mut(scene_entity).insert((
                    model.model.occlusion_bounds(),
                    model,
                    TfxFeatureRenderer::DynamicObjects,
                ));
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
                    None,
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
