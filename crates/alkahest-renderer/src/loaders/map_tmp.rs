use std::{
    hash::Hash,
    io::{Cursor, Read, Seek, SeekFrom},
};

use alkahest_data::{
    entity::{SEntity, Unk808072c5},
    map::{
        SBubbleParent, SLightCollection, SMapDataTable, SShadowingLight, Unk808068d4, Unk80806aa7,
        Unk80806ef4, Unk8080714b,
    },
};
use alkahest_pm::package_manager;
use anyhow::Context;
use binrw::BinReaderExt;
use destiny_pkg::TagHash;
use glam::{Mat4, Quat, Vec3};
use itertools::multizip;
use rustc_hash::FxHashSet;
use tiger_parse::{Endian, PackageManagerExt, TigerReadable};

use crate::{
    ecs::{
        components::ResourceOrigin,
        dynamic_geometry::{DynamicModel, DynamicModelComponent},
        light::LightRenderer,
        static_geometry::{StaticInstance, StaticInstances, StaticModel},
        terrain::TerrainPatches,
        transform::{Transform, TransformFlags},
        Scene,
    },
    gpu::{buffer::ConstantBuffer, SharedGpuContext},
    loaders::AssetManager,
};

pub fn load_map(
    gctx: SharedGpuContext,
    asset_manager: &mut AssetManager,
    tag: TagHash,
) -> anyhow::Result<Scene> {
    let bubble_parent = package_manager()
        .read_tag_struct::<SBubbleParent>(tag)
        .context("Failed to read SBubbleParent")?;

    let mut scene = Scene::new();

    let mut data_tables = FxHashSet::<TagHash>::default();
    for map_container in &bubble_parent.child_map.map_resources {
        for table in &map_container.data_tables {
            data_tables.insert(*table);
        }
    }

    for table_hash in data_tables {
        let table_data = package_manager().read_tag(table_hash).unwrap();
        let mut cur = Cursor::new(&table_data);
        let table = TigerReadable::read_ds(&mut cur)?;

        load_datatable_into_scene(
            &table,
            table_hash,
            &mut cur,
            &mut scene,
            gctx.clone(),
            asset_manager,
            ResourceOrigin::Map,
            0,
        )
        .context("Failed to load datatable")?;
    }

    Ok(scene)
}

// clippy: asset system will fix this lint on it's own (i hope)
#[allow(clippy::too_many_arguments)]
fn load_datatable_into_scene<R: Read + Seek>(
    table: &SMapDataTable,
    _table_hash: TagHash,
    table_data: &mut R,
    scene: &mut Scene,
    gctx: SharedGpuContext,
    asset_manager: &mut AssetManager,
    _resource_origin: ResourceOrigin,
    _group_id: u32,
) -> anyhow::Result<()> {
    for data in &table.data_entries {
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
                let preheader: Unk80806ef4 =
                    package_manager().read_tag_struct(preheader_tag).unwrap();

                for s in &preheader.instances.instance_groups {
                    let mesh_tag = preheader.instances.statics[s.static_index as usize];
                    let model = StaticModel::load(asset_manager, mesh_tag)
                        .context("Failed to load StaticModel")?;

                    let transforms = &preheader.instances.transforms
                        [s.instance_start as usize..(s.instance_start + s.instance_count) as usize];

                    let parent = scene.reserve_entity();
                    let mut instances = vec![];

                    for transform in transforms.iter() {
                        let transform = Transform {
                            translation: transform.translation,
                            rotation: transform.rotation,
                            scale: Vec3::splat(transform.scale.x),
                            flags: TransformFlags::empty(),
                        };

                        let entity = scene.spawn((transform, StaticInstance { parent }));
                        instances.push(entity);
                    }

                    scene.insert_one(
                        parent,
                        StaticInstances {
                            cbuffer: ConstantBuffer::create_array_init(
                                gctx.clone(),
                                &vec![0u8; 32 + 64 * instances.len()],
                            )?,
                            instances,
                            model,
                        },
                    )?;
                }
            }
            // D2Class_7D6C8080 (terrain)
            0x80806c7d => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset))
                    .unwrap();

                let terrain_resource: Unk8080714b = TigerReadable::read_ds(table_data).unwrap();

                scene.spawn((TerrainPatches::load(
                    gctx.clone(),
                    asset_manager,
                    terrain_resource.terrain,
                )
                .context("Failed to load terrain patches")?,));
            }
            0x80806aa3 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();
                let tag: TagHash = TigerReadable::read_ds(table_data).unwrap();
                if tag.is_none() {
                    continue;
                }

                let header: Unk80806aa7 = package_manager().read_tag_struct(tag).unwrap();

                for (unk8, unk18, _unk28) in itertools::multizip((
                    header.unk8.iter(),
                    header.unk18.iter(),
                    header.unk28.iter(),
                )) {
                    if unk8.bounds != unk18.bb {
                        warn!(
                            "Bounds mismatch in Unk80806aa3: {:?} != {:?}",
                            unk8.bounds, unk18.bb
                        );
                    }

                    scene.spawn((
                        Transform::from_mat4(Mat4::from_cols_array(&unk8.transform)),
                        DynamicModelComponent {
                            model: DynamicModel::load(
                                asset_manager,
                                unk8.unk60.entity_model,
                                vec![],
                                vec![],
                            )
                            .context("Failed to load background dynamic model")?,
                            cbuffer: ConstantBuffer::create(gctx.clone(), None)?,
                        },
                    ));
                }
            }
            0x808068d4 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset))
                    .unwrap();

                let d: Unk808068d4 = TigerReadable::read_ds(table_data)?;

                scene.spawn((
                    transform,
                    DynamicModelComponent {
                        model: DynamicModel::load(asset_manager, d.entity_model, vec![], vec![])
                            .context("Failed to load background dynamic model")?,
                        cbuffer: ConstantBuffer::create(gctx.clone(), None)?,
                    },
                ));
            }
            0x80806a63 => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();
                let tag: TagHash = table_data.read_le().unwrap();
                if !tag.is_some() {
                    continue;
                }

                let header: SLightCollection = package_manager().read_tag_struct(tag).unwrap();

                for (_i, (transform, light, bounds)) in
                    multizip((header.unk40, header.unk30, &header.occlusion_bounds.bounds))
                        .enumerate()
                {
                    scene.spawn((
                        Transform {
                            translation: Vec3::new(
                                transform.translation.x,
                                transform.translation.y,
                                transform.translation.z,
                            ),
                            rotation: Quat::from_xyzw(
                                transform.rotation.x,
                                transform.rotation.y,
                                transform.rotation.z,
                                transform.rotation.w,
                            ),
                            ..Default::default()
                        },
                        LightRenderer::load(gctx.clone(), asset_manager, &light)
                            .context("Failed to load light")?,
                        light,
                        bounds.bb,
                    ));
                }
            }
            0x80806c5e => {
                table_data
                    .seek(SeekFrom::Start(data.data_resource.offset + 16))
                    .unwrap();
                let tag: TagHash = table_data.read_le().unwrap();
                println!("light {tag}");
                let light: SShadowingLight = package_manager().read_tag_struct(tag)?;
                println!("{light:#X?}");

                scene.spawn((
                    transform,
                    LightRenderer::load_shadowing(gctx.clone(), asset_manager, &light)
                        .context("Failed to load shadowing light")?,
                    light,
                ));
            }
            u => {
                if u != u32::MAX {
                    warn!("Unknown resource type {u:08X}");
                }
                let entity_hash = data.entity.hash32();
                if entity_hash.is_none() {
                    continue;
                }

                let header = package_manager()
                    .read_tag_struct::<SEntity>(entity_hash)
                    .context("Failed to read SEntity")?;
                for e in &header.entity_resources {
                    match e.unk0.unk10.resource_type {
                        0x80806d8a => {
                            let mut cur = Cursor::new(package_manager().read_tag(e.unk0.hash())?);
                            cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x224))?;
                            let model_hash: TagHash =
                                TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                            cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x3c0))?;
                            let entity_material_map: Vec<Unk808072c5> =
                                TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;
                            cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x400))?;
                            let materials: Vec<TagHash> =
                                TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                            scene.spawn((
                                transform,
                                DynamicModelComponent {
                                    model: DynamicModel::load(
                                        asset_manager,
                                        model_hash,
                                        entity_material_map,
                                        materials,
                                    )
                                    .context("Failed to load background dynamic model")?,
                                    cbuffer: ConstantBuffer::create(gctx.clone(), None)?,
                                },
                            ));
                        }
                        u => {
                            debug!(
                                "\t- Unknown entity resource type {:08X}/{:08X} (table {})",
                                u.to_be(),
                                e.unk0.unk10.resource_type.to_be(),
                                e.unk0.hash()
                            )
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
