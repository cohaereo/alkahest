// ! Temporary file to mitigate performance issues in some IDEs while we figure out loading routines

use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Seek, SeekFrom},
    sync::Arc,
};

use crate::{
    map_resources::{Unk80806d19, Unk808085c2, Unk80808cb7},
    util::RwLock,
};
use anyhow::Context;
use binrw::BinReaderExt;
use destiny_pkg::TagHash;
use glam::{Mat4, Quat, Vec3, Vec4};
use itertools::Itertools;
use nohash_hasher::IntMap;
use windows::Win32::Graphics::{
    Direct3D::WKPDID_D3DDebugObjectName,
    Direct3D11::{ID3D11PixelShader, ID3D11SamplerState, ID3D11VertexShader},
};

use crate::{
    dxbc::{get_input_signature, get_output_signature, DxbcHeader, DxbcInputType},
    entity::{Unk808072c5, Unk808073a5, Unk80809c0f},
    map::{ExtendedHash, MapData, Unk80806ef4, Unk8080714f, Unk80807dae, Unk80808a54},
    map_resources::{
        MapResource, ResourcePoint, Unk80806aa7, Unk80806b7f, Unk80806c65, Unk80806e68, Unk8080714b,
    },
    material::{Material, Unk808071e8},
    packages::package_manager,
    render::{
        renderer::Renderer, scopes::ScopeRigidModel, vertex_layout::InputElement, ConstantBuffer,
        DeviceContextSwapchain, EntityRenderer, InstancedRenderer, StaticModel, TerrainRenderer,
    },
    statics::{Unk808071a7, Unk8080966d},
    structure::{TablePointer, Tag},
    types::AABB,
};

pub async fn load_maps(
    dcs: Arc<DeviceContextSwapchain>,
    renderer: Arc<RwLock<Renderer>>,
    map_hashes: Vec<TagHash>,
    stringmap: Arc<IntMap<u32, String>>,
) -> anyhow::Result<LoadMapsData> {
    let mut static_map: IntMap<TagHash, Arc<StaticModel>> = Default::default();
    let mut material_map: IntMap<TagHash, Material> = Default::default();
    let mut vshader_map: IntMap<TagHash, (ID3D11VertexShader, Vec<InputElement>, Vec<u8>)> =
        Default::default();
    let mut pshader_map: IntMap<TagHash, (ID3D11PixelShader, Vec<InputElement>)> =
        Default::default();
    let mut sampler_map: IntMap<u64, ID3D11SamplerState> = Default::default();

    let mut maps: Vec<(TagHash, MapData)> = vec![];
    let mut terrain_headers = vec![];
    for hash in map_hashes {
        let renderer = renderer.read();
        let _span = debug_span!("Load map", %hash).entered();
        let think: Unk80807dae = package_manager().read_tag_struct(hash).unwrap();

        // if stringmap.get(&think.map_name.0) != Some(&"Quagmire".to_string()) {
        //     continue;
        // }

        // TODO(cohae): obviously dont do lights like this
        // First lights reserved for camera light and directional light
        let mut point_lights = vec![Vec4::ZERO, Vec4::ZERO];
        let mut placement_groups = vec![];
        let mut resource_points = vec![];
        let mut terrains = vec![];

        let mut unknown_root_resources: IntMap<u32, ()> = IntMap::default();
        for res in &think.child_map.map_resources {
            let thing2: Unk80808a54 = package_manager()
                .read_tag_struct(res.hash32().unwrap())
                .unwrap();

            for table in &thing2.data_tables {
                let table_data = package_manager().read_tag(table.tag()).unwrap();
                let mut cur = Cursor::new(&table_data);

                for data in &table.data_entries {
                    if data.data_resource.is_valid {
                        match data.data_resource.resource_type {
                            // D2Class_C96C8080 (placement)
                            0x80806cc9 => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                                    .unwrap();
                                let preheader_tag: TagHash = cur.read_le().unwrap();
                                let preheader: Unk80806ef4 =
                                    package_manager().read_tag_struct(preheader_tag).unwrap();

                                placement_groups.push(preheader.placement_group);
                            }
                            // D2Class_7D6C8080 (terrain)
                            0x80806c7d => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset))
                                    .unwrap();

                                let terrain_resource: Unk8080714b = cur.read_le().unwrap();
                                let terrain: Unk8080714f = package_manager()
                                    .read_tag_struct(terrain_resource.terrain)
                                    .unwrap();

                                for p in &terrain.mesh_parts {
                                    if p.material.is_valid() {
                                        material_map.insert(
                                            p.material,
                                            Material::load(
                                                &renderer,
                                                package_manager().read_tag_struct(p.material)?,
                                                p.material,
                                                true,
                                            ),
                                        );
                                    }
                                }

                                terrain_headers.push((terrain_resource.terrain, terrain));
                                terrains.push(terrain_resource.terrain);
                            }
                            // Cubemap volume
                            0x80806695 => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset))
                                    .unwrap();

                                let cubemap_volume: Unk80806b7f = cur.read_le().unwrap();
                                let extents_center = Vec4::new(
                                    data.translation.x,
                                    data.translation.y,
                                    data.translation.z,
                                    data.translation.w,
                                );
                                let extents = Vec4::new(
                                    cubemap_volume.cubemap_extents.x,
                                    cubemap_volume.cubemap_extents.y,
                                    cubemap_volume.cubemap_extents.z,
                                    cubemap_volume.cubemap_extents.w,
                                );

                                let volume_min = extents_center - extents / 2.0;
                                let volume_max = extents_center + extents / 2.0;

                                renderer.render_data.load_texture(ExtendedHash::Hash32(
                                    cubemap_volume.cubemap_texture,
                                ));

                                resource_points.push(ResourcePoint {
                                    translation: extents_center,
                                    rotation: Quat::from_xyzw(
                                        data.rotation.x,
                                        data.rotation.y,
                                        data.rotation.z,
                                        data.rotation.w,
                                    ),
                                    entity: data.entity,
                                    has_havok_data: is_physics_entity(data.entity),
                                    world_id: data.world_id,
                                    resource_type: data.data_resource.resource_type,
                                    resource: MapResource::CubemapVolume(
                                        Box::new(cubemap_volume),
                                        AABB {
                                            min: volume_min.truncate().into(),
                                            max: volume_max.truncate().into(),
                                        },
                                    ),
                                });
                            }
                            0x808067b5 => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                                    .unwrap();
                                let tag: TagHash = cur.read_le().unwrap();
                                resource_points.push(ResourcePoint {
                                    translation: Vec4::new(
                                        data.translation.x,
                                        data.translation.y,
                                        data.translation.z,
                                        data.translation.w,
                                    ),
                                    rotation: Quat::from_xyzw(
                                        data.rotation.x,
                                        data.rotation.y,
                                        data.rotation.z,
                                        data.rotation.w,
                                    ),
                                    entity: data.entity,
                                    has_havok_data: is_physics_entity(data.entity),
                                    world_id: data.world_id,
                                    resource_type: data.data_resource.resource_type,
                                    resource: MapResource::Unk808067b5(tag),
                                });
                            }
                            // Decal collection
                            0x80806955 => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                                    .unwrap();
                                let tag: TagHash = cur.read_le().unwrap();
                                if !tag.is_valid() {
                                    continue;
                                }

                                let header: Unk80806e68 =
                                    package_manager().read_tag_struct(tag).unwrap();

                                for inst in &header.instances {
                                    for i in inst.start..(inst.start + inst.count) {
                                        let transform = header.transforms[i as usize];
                                        resource_points.push(ResourcePoint {
                                            translation: Vec4::new(
                                                transform.x,
                                                transform.y,
                                                transform.z,
                                                transform.w,
                                            ),
                                            rotation: Quat::from_xyzw(
                                                data.rotation.x,
                                                data.rotation.y,
                                                data.rotation.z,
                                                data.rotation.w,
                                            ),
                                            entity: data.entity,
                                            has_havok_data: is_physics_entity(data.entity),
                                            world_id: data.world_id,
                                            resource_type: data.data_resource.resource_type,
                                            resource: MapResource::Decal {
                                                material: inst.material,
                                                scale: transform.w,
                                            },
                                        })
                                    }
                                }
                            }
                            // // Unknown, every element has a mesh (material+index+vertex) and the required transforms
                            // 0x80806df1 => {
                            //     cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                            //         .unwrap();
                            //     let tag: TagHash = cur.read_le().unwrap();
                            //     if !tag.is_valid() {
                            //         continue;
                            //     }

                            //     let header: Unk80806df3 =
                            //         package_manager().read_tag_struct(tag).unwrap();

                            //     for p in &header.unk8 {
                            //         resource_points.push(ResourcePoint {
                            //             translation: Vec4::new(
                            //                 p.translation.x,
                            //                 p.translation.y,
                            //                 p.translation.z,
                            //                 p.translation.w,
                            //             ),
                            //             rotation: Quat::IDENTITY,
                            //             entity: data.entity,
                            //             resource_type: data.data_resource.resource_type,
                            //             resource: MapResource::Unk80806df1,
                            //         });
                            //     }
                            // }
                            // // Unknown, structure seems like that of an octree
                            // 0x80806f38 => {
                            //     cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                            //         .unwrap();
                            //     let tag: TagHash = cur.read_le().unwrap();
                            //     if !tag.is_valid() {
                            //         continue;
                            //     }

                            //     let header: Unk80807268 =
                            //         package_manager().read_tag_struct(tag).unwrap();

                            //     for p in &header.unk50 {
                            //         resource_points.push(ResourcePoint {
                            //             translation: Vec4::new(
                            //                 p.unk0.x, p.unk0.y, p.unk0.z, p.unk0.w,
                            //             ),
                            //             rotation: Quat::IDENTITY,
                            //             entity: data.entity,
                            //             resource_type: data.data_resource.resource_type,
                            //             resource: MapResource::Unk80806f38,
                            //         });
                            //     }
                            // }
                            // 0x80809160 => {
                            //     cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                            //         .unwrap();
                            //     let tag: TagHash = cur.read_le().unwrap();
                            //     if !tag.is_valid() {
                            //         continue;
                            //     }

                            //     let header: Unk80809162 =
                            //         package_manager().read_tag_struct(tag).unwrap();

                            //     for p in &header.unk8 {
                            //         resource_points.push(ResourcePoint {
                            //             translation: Vec4::new(
                            //                 p.unk10.x, p.unk10.y, p.unk10.z, p.unk10.w,
                            //             ),
                            //             rotation: Quat::IDENTITY,
                            //             entity: data.entity,
                            //             resource_type: data.data_resource.resource_type,
                            //             resource: MapResource::RespawnPoint,
                            //         });
                            //     }
                            // }
                            // // (ambient) sound source
                            // 0x80806b5b => {
                            //     cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                            //         .unwrap();
                            //     let tag: TagHash = cur.read_le().unwrap();
                            //     if !tag.is_valid() {
                            //         continue;
                            //     }

                            //     let header: Unk80809802 =
                            //         package_manager().read_tag_struct(tag).unwrap();

                            //     resource_points.push(ResourcePoint {
                            //         translation: Vec4::new(
                            //             data.translation.x,
                            //             data.translation.y,
                            //             data.translation.z,
                            //             data.translation.w,
                            //         ),
                            //         rotation: Quat::IDENTITY,
                            //         entity: data.entity,
                            //         resource_type: data.data_resource.resource_type,
                            //         resource: MapResource::AmbientSound(header),
                            //     });
                            // }
                            0x80806aa3 => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                                    .unwrap();
                                let tag: TagHash = cur.read_le().unwrap();
                                if !tag.is_valid() {
                                    continue;
                                }

                                let header: Unk80806aa7 =
                                    package_manager().read_tag_struct(tag).unwrap();

                                for (unk8, unk18, _unk28) in itertools::multizip((
                                    header.unk8.iter(),
                                    header.unk18.iter(),
                                    header.unk28.iter(),
                                )) {
                                    resource_points.push(ResourcePoint {
                                        translation: Vec4::new(
                                            unk8.bounds_center.x,
                                            unk8.bounds_center.y,
                                            unk8.bounds_center.z,
                                            unk8.bounds_center.w,
                                        ),
                                        rotation: Quat::IDENTITY,
                                        entity: data.entity,
                                        has_havok_data: is_physics_entity(data.entity),
                                        world_id: data.world_id,
                                        resource_type: data.data_resource.resource_type,
                                        resource: MapResource::Unk80806aa3(unk18.bb),
                                    });
                                }
                            }
                            0x80806a63 => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                                    .unwrap();
                                let tag: TagHash = cur.read_le().unwrap();
                                if !tag.is_valid() {
                                    continue;
                                }

                                let header: Unk80806c65 =
                                    package_manager().read_tag_struct(tag).unwrap();

                                for (transform, _unk) in header.unk40.iter().zip(&header.unk30) {
                                    resource_points.push(ResourcePoint {
                                        translation: Vec4::new(
                                            transform.translation.x,
                                            transform.translation.y,
                                            transform.translation.z,
                                            transform.translation.w,
                                        ),
                                        rotation: Quat::from_xyzw(
                                            transform.rotation.x,
                                            transform.rotation.y,
                                            transform.rotation.z,
                                            transform.rotation.w,
                                        ),
                                        entity: data.entity,
                                        has_havok_data: is_physics_entity(data.entity),
                                        world_id: data.world_id,
                                        resource_type: data.data_resource.resource_type,
                                        resource: MapResource::Light,
                                    });

                                    point_lights.push(Vec4::new(
                                        transform.translation.x,
                                        transform.translation.y,
                                        transform.translation.z,
                                        transform.translation.w,
                                    ));
                                }
                            }
                            0x80808cb5 => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                                    .unwrap();
                                let tag: TagHash = cur.read_le().unwrap();
                                if !tag.is_valid() {
                                    continue;
                                }

                                let header: Unk80808cb7 =
                                    package_manager().read_tag_struct(tag).unwrap();

                                for transform in header.unk8.iter() {
                                    resource_points.push(ResourcePoint {
                                        translation: Vec4::new(
                                            transform.translation.x,
                                            transform.translation.y,
                                            transform.translation.z,
                                            transform.translation.w,
                                        ),
                                        rotation: Quat::IDENTITY,
                                        entity: data.entity,
                                        has_havok_data: is_physics_entity(data.entity),
                                        world_id: data.world_id,
                                        resource_type: data.data_resource.resource_type,
                                        resource: MapResource::RespawnPoint,
                                    });
                                }
                            }
                            0x808085c0 => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                                    .unwrap();
                                let tag: TagHash = cur.read_le().unwrap();
                                if !tag.is_valid() {
                                    continue;
                                }

                                let header: Unk808085c2 =
                                    package_manager().read_tag_struct(tag).unwrap();

                                for transform in header.unk8.iter() {
                                    resource_points.push(ResourcePoint {
                                        translation: Vec4::new(
                                            transform.translation.x,
                                            transform.translation.y,
                                            transform.translation.z,
                                            transform.translation.w,
                                        ),
                                        rotation: Quat::IDENTITY,
                                        entity: data.entity,
                                        has_havok_data: is_physics_entity(data.entity),
                                        world_id: data.world_id,
                                        resource_type: data.data_resource.resource_type,
                                        resource: MapResource::Unk808085c0,
                                    });
                                }
                            }
                            0x8080684d => {
                                // TODO(cohae): Collection of havok files
                            }
                            0x80806a40 => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                                    .unwrap();
                                let tag: TagHash = cur.read_le().unwrap();
                                if !tag.is_valid() {
                                    continue;
                                }

                                let header: Unk80806d19 =
                                    package_manager().read_tag_struct(tag).unwrap();

                                for transform in header.unk50.iter() {
                                    resource_points.push(ResourcePoint {
                                        translation: Vec4::new(
                                            transform.translation.x,
                                            transform.translation.y,
                                            transform.translation.z,
                                            transform.translation.w,
                                        ),
                                        rotation: Quat::IDENTITY,
                                        entity: data.entity,
                                        has_havok_data: is_physics_entity(data.entity),
                                        world_id: data.world_id,
                                        resource_type: data.data_resource.resource_type,
                                        resource: MapResource::Unk80806a40,
                                    });
                                }
                            }
                            u => {
                                // println!("{data:x?}");
                                if data.translation.x == 0.0
                                    && data.translation.y == 0.0
                                    && data.translation.z == 0.0
                                    && !unknown_root_resources.contains_key(&u)
                                {
                                    warn!("World origin resource {} is not parsed! Resource points might be missing (table {})", TagHash(u), table.tag());
                                    unknown_root_resources.insert(u, ());
                                }

                                debug!(
                                    "Skipping unknown resource type {u:x} {:?} (table file {:?})",
                                    data.translation,
                                    table.tag()
                                );
                                resource_points.push(ResourcePoint {
                                    translation: Vec4::new(
                                        data.translation.x,
                                        data.translation.y,
                                        data.translation.z,
                                        data.translation.w,
                                    ),
                                    rotation: Quat::from_xyzw(
                                        data.rotation.x,
                                        data.rotation.y,
                                        data.rotation.z,
                                        data.rotation.w,
                                    ),
                                    entity: data.entity,
                                    has_havok_data: is_physics_entity(data.entity),
                                    world_id: data.world_id,
                                    resource_type: data.data_resource.resource_type,
                                    resource: MapResource::Unknown(
                                        data.data_resource.resource_type,
                                        data.world_id,
                                        data.entity,
                                        data.data_resource,
                                        table.tag(),
                                    ),
                                });
                            }
                        };
                    } else {
                        resource_points.push(ResourcePoint {
                            translation: Vec4::new(
                                data.translation.x,
                                data.translation.y,
                                data.translation.z,
                                data.translation.w,
                            ),
                            rotation: Quat::from_xyzw(
                                data.rotation.x,
                                data.rotation.y,
                                data.rotation.z,
                                data.rotation.w,
                            ),
                            entity: data.entity,
                            has_havok_data: is_physics_entity(data.entity),
                            world_id: data.world_id,
                            resource_type: u32::MAX,
                            resource: MapResource::Entity(data.entity, data.world_id),
                        });
                    }
                }
            }
        }

        let map_name = stringmap
            .get(&think.map_name.0)
            .cloned()
            .unwrap_or(format!("[MissingString_{:08x}]", think.map_name.0));
        info!(
            "Map {:x?} '{map_name}' - {} placement groups, {} decals",
            think.map_name,
            placement_groups.len(),
            resource_points
                .iter()
                .filter(|r| r.resource.is_decal())
                .count()
        );

        let cb_composite_lights =
            ConstantBuffer::<Vec4>::create_array_init(dcs.clone(), &point_lights)?;

        maps.push((
            hash,
            MapData {
                hash,
                name: map_name,
                placement_groups,
                resource_points: resource_points
                    .into_iter()
                    .map(|rp| {
                        let cb = ConstantBuffer::create(dcs.clone(), None).unwrap();

                        (rp, cb)
                    })
                    .collect(),
                terrains,
                lights: point_lights,
                lights_cbuffer: cb_composite_lights,
            },
        ))
    }

    let to_load_entities: HashSet<ExtendedHash> = maps
        .iter()
        .flat_map(|(_, v)| v.resource_points.iter().map(|(r, _)| r.entity))
        .filter(|v| v.is_valid())
        .collect();

    let mut entity_renderers: IntMap<u64, EntityRenderer> = Default::default();
    for te in &to_load_entities {
        let renderer = renderer.read();
        if let Some(nh) = te.hash32() {
            let _span = debug_span!("Load entity", hash = %nh).entered();
            let Ok(header) = package_manager().read_tag_struct::<Unk80809c0f>(nh) else {
                error!("Could not load entity {nh} ({te:?})");
                continue;
            };
            debug!("Loading entity {nh}");
            for e in &header.entity_resources {
                match e.unk0.unk10.resource_type {
                    0x80806d8a => {
                        debug!(
                            "\t- EntityModel {:08x}/{}",
                            e.unk0.unk18.resource_type.to_be(),
                            e.unk0.unk10.resource_type.to_be(),
                        );
                        let mut cur = Cursor::new(package_manager().read_tag(e.unk0.tag())?);
                        cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x224))?;
                        let model: Tag<Unk808073a5> = cur.read_le()?;
                        cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x3c0))?;
                        let entity_material_map: TablePointer<Unk808072c5> = cur.read_le()?;
                        cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x400))?;
                        let materials: TablePointer<Tag<Unk808071e8>> = cur.read_le()?;

                        for m in &materials {
                            material_map.insert(
                                m.tag(),
                                Material::load(&renderer, m.0.clone(), m.tag(), true),
                            );
                        }

                        for m in &model.meshes {
                            for p in &m.parts {
                                if p.material.is_valid() {
                                    material_map.insert(
                                        p.material,
                                        Material::load(
                                            &renderer,
                                            package_manager().read_tag_struct(p.material)?,
                                            p.material,
                                            true,
                                        ),
                                    );
                                }
                            }
                        }

                        if entity_renderers
                            .insert(
                                te.key(),
                                debug_span!("load EntityRenderer").in_scope(|| {
                                    EntityRenderer::load(
                                        model.0,
                                        entity_material_map.to_vec(),
                                        materials.iter().map(|m| m.tag()).collect_vec(),
                                        &renderer,
                                        &dcs,
                                    )
                                })?,
                            )
                            .is_some()
                        {
                            error!("More than 1 model was loaded for entity {nh}");
                        }

                        // println!(" - EntityModel {model:?}");
                    }
                    u => {
                        if nh.0 == 0x80e792e1 {
                            info!(
                                "\t- Unknown entity resource type {:08X}/{:08X} (table {})",
                                u.to_be(),
                                e.unk0.unk10.resource_type.to_be(),
                                e.unk0.tag()
                            )
                        }

                        debug!(
                            "\t- Unknown entity resource type {:08X}/{:08X} (table {})",
                            u.to_be(),
                            e.unk0.unk10.resource_type.to_be(),
                            e.unk0.tag()
                        )
                    }
                }
            }

            if !entity_renderers.contains_key(&te.key()) {
                warn!("Entity {nh} does not contain any geometry!");
            }
        }
    }

    info!(
        "Found {} entity models ({} entities)",
        entity_renderers.len(),
        to_load_entities.len()
    );

    // TODO(cohae): Maybe not the best idea?
    info!("Updating resource constant buffers");
    for (_, m) in &mut maps {
        for (rp, cb) in &mut m.resource_points {
            if let Some(ent) = entity_renderers.get(&rp.entity.key()) {
                let mm = Mat4::from_scale_rotation_translation(
                    Vec3::splat(rp.translation.w),
                    rp.rotation.inverse(),
                    Vec3::ZERO,
                );
                let model_matrix = Mat4::from_cols(
                    mm.x_axis.truncate().extend(rp.translation.x),
                    mm.y_axis.truncate().extend(rp.translation.y),
                    mm.z_axis.truncate().extend(rp.translation.z),
                    rp.translation,
                );
                let alt_matrix = Mat4::from_cols(
                    Vec3::ONE.extend(rp.translation.x),
                    Vec3::ONE.extend(rp.translation.y),
                    Vec3::ONE.extend(rp.translation.z),
                    Vec4::W,
                );

                *cb = ConstantBuffer::create(
                    dcs.clone(),
                    Some(&ScopeRigidModel {
                        mesh_to_world: model_matrix.transpose(),
                        position_scale: ent.mesh_scale(),
                        position_offset: ent.mesh_offset(),
                        texcoord0_scale_offset: ent.texcoord_transform(),
                        dynamic_sh_ao_values: Vec4::new(1.0, 1.0, 1.0, 0.0),
                        unk8: [alt_matrix; 8],
                    }),
                )
                .unwrap();
            }
        }
    }

    let mut placement_renderers: IntMap<u32, (Unk8080966d, Vec<InstancedRenderer>)> =
        IntMap::default();

    let mut to_load: HashMap<TagHash, ()> = Default::default();
    let mut to_load_samplers: HashSet<ExtendedHash> = Default::default();
    for (_, m) in &maps {
        for placements in m.placement_groups.iter() {
            for v in &placements.statics {
                to_load.insert(*v, ());
            }
            placement_renderers.insert(placements.tag().0, (placements.0.clone(), vec![]));
        }
    }

    if placement_renderers.is_empty() {
        panic!("No map placements found in package");
    }

    let mut terrain_renderers: IntMap<u32, TerrainRenderer> = Default::default();
    info!("Loading {} terrain renderers", terrain_headers.len());
    info_span!("Loading terrain").in_scope(|| {
        let renderer = renderer.read();
        for (t, header) in terrain_headers.into_iter() {
            for t in &header.mesh_groups {
                renderer
                    .render_data
                    .load_texture(ExtendedHash::Hash32(t.dyemap));
            }

            match TerrainRenderer::load(header, dcs.clone(), &renderer) {
                Ok(r) => {
                    terrain_renderers.insert(t.0, r);
                }
                Err(e) => {
                    error!("Failed to load terrain: {e}");
                }
            }
        }
    });

    let to_load_statics: Vec<TagHash> = to_load.keys().cloned().collect();

    info!("Loading statics");
    info_span!("Loading statics").in_scope(|| {
        let renderer = renderer.read();
        for almostloadable in &to_load_statics {
            let mheader: Unk808071a7 = debug_span!("load tag Unk808071a7")
                .in_scope(|| package_manager().read_tag_struct(*almostloadable).unwrap());
            for m in &mheader.materials {
                if m.is_valid()
                    && !material_map.contains_key(m)
                    && !renderer.render_data.data().materials.contains_key(m)
                {
                    material_map.insert(
                        *m,
                        Material::load(
                            &renderer,
                            package_manager().read_tag_struct(*m).unwrap(),
                            *m,
                            true,
                        ),
                    );
                }
            }
            for m in &mheader.unk20 {
                let m = m.material;
                if m.is_valid()
                    && !material_map.contains_key(&m)
                    && !renderer.render_data.data().materials.contains_key(&m)
                {
                    material_map.insert(
                        m,
                        Material::load(
                            &renderer,
                            package_manager().read_tag_struct(m).unwrap(),
                            m,
                            true,
                        ),
                    );
                }
            }

            debug_span!("load StaticModel").in_scope(|| {
                match StaticModel::load(mheader, &renderer, *almostloadable) {
                    Ok(model) => {
                        static_map.insert(*almostloadable, Arc::new(model));
                    }
                    Err(e) => {
                        error!(model = ?almostloadable, "Failed to load model: {e}");
                    }
                }
            });
        }
    });

    info!("Loaded {} statics", static_map.len());

    info_span!("Constructing instance renderers").in_scope(|| {
        let mut total_instance_data = 0;
        for (placements, renderers) in placement_renderers.values_mut() {
            for instance in &placements.instances {
                if let Some(model_hash) =
                    placements.statics.iter().nth(instance.static_index as _)
                {
                    let _span =
                        debug_span!("Draw static instance", count = instance.instance_count, model = ?model_hash)
                            .entered();

                    if let Some(model) = static_map.get(model_hash) {
                        let transforms = &placements.transforms[instance.instance_start
                            as usize
                            ..(instance.instance_start + instance.instance_count) as usize];

                        renderers.push(InstancedRenderer::load(model.clone(), transforms, dcs.clone()).unwrap());
                    } else {
                        error!("Couldn't get static model {model_hash}");
                    }

                    total_instance_data += instance.instance_count as usize * 16 * 4;
                } else {
                    error!("Couldn't get instance static #{}", instance.static_index);
                }
            }
        }
        debug!("Total instance data: {}kb", total_instance_data / 1024);
    });

    info_span!("Loading shaders").in_scope(|| {
        for (t, m) in material_map.iter() {
            for sampler in m.vs_samplers.iter().chain(m.ps_samplers.iter()) {
                to_load_samplers.insert(*sampler);
            }

            if let Ok(v) = package_manager().get_entry(m.vertex_shader) {
                let _span = debug_span!("load vshader", shader = ?m.vertex_shader).entered();

                vshader_map.entry(m.vertex_shader).or_insert_with(|| {
                    let vs_data = package_manager().read_tag(v.reference).unwrap();

                    let mut vs_cur = Cursor::new(&vs_data);
                    let dxbc_header: DxbcHeader = vs_cur.read_le().unwrap();
                    let input_sig = get_input_signature(&mut vs_cur, &dxbc_header).unwrap();

                    let layout_converted = input_sig
                        .elements
                        .iter()
                        .map(|e| {
                            InputElement::from_dxbc(
                                e,
                                e.component_type == DxbcInputType::Float,
                                false,
                            )
                        })
                        .collect_vec();

                    unsafe {
                        let v = dcs
                            .device
                            .CreateVertexShader(&vs_data, None)
                            .context("Failed to load vertex shader")
                            .unwrap();

                        let name = format!("VS {:?} (mat {})\0", m.vertex_shader, t);
                        v.SetPrivateData(
                            &WKPDID_D3DDebugObjectName,
                            name.len() as u32 - 1,
                            Some(name.as_ptr() as _),
                        )
                        .expect("Failed to set VS name");

                        // let input_layout = dcs.device.CreateInputLayout(&layout, &vs_data).unwrap();
                        // let layout_string = layout_converted
                        //     .iter()
                        //     .enumerate()
                        //     .map(|(i, e)| {
                        //         format!(
                        //             "\t{}{} v{i} : {}{}",
                        //             e.component_type,
                        //             e.component_count,
                        //             e.semantic_type.to_pcstr().display(),
                        //             e.semantic_index
                        //         )
                        //     })
                        //     .join("\n");

                        // error!(
                        //     "Failed to load vertex layout for VS {:?}, layout:\n{}\n",
                        //     m.vertex_shader, layout_string
                        // );

                        (v, layout_converted, vs_data)
                    }
                });
            }

            // return Ok(());

            if let Ok(v) = package_manager().get_entry(m.pixel_shader) {
                let _span = debug_span!("load pshader", shader = ?m.pixel_shader).entered();

                pshader_map.entry(m.pixel_shader).or_insert_with(|| {
                    let ps_data = package_manager().read_tag(v.reference).unwrap();

                    let mut ps_cur = Cursor::new(&ps_data);
                    let dxbc_header: DxbcHeader = ps_cur.read_le().unwrap();
                    let output_sig = get_output_signature(&mut ps_cur, &dxbc_header).unwrap();

                    let layout_converted = output_sig
                        .elements
                        .iter()
                        .map(|e| {
                            InputElement::from_dxbc(
                                e,
                                e.component_type == DxbcInputType::Float,
                                false,
                            )
                        })
                        .collect_vec();

                    unsafe {
                        let v = dcs
                            .device
                            .CreatePixelShader(&ps_data, None)
                            .context("Failed to load pixel shader")
                            .unwrap();

                        let name = format!("PS {:?} (mat {})\0", m.pixel_shader, t);
                        v.SetPrivateData(
                            &WKPDID_D3DDebugObjectName,
                            name.len() as u32 - 1,
                            Some(name.as_ptr() as _),
                        )
                        .expect("Failed to set VS name");

                        (v, layout_converted)
                    }
                });
            }
        }
    });

    info!(
        "Loaded {} vertex shaders, {} pixel shaders",
        vshader_map.len(),
        pshader_map.len()
    );

    info!("Loaded {} materials", material_map.len());
    {
        let renderer = renderer.read();
        for m in material_map.values() {
            for t in m.ps_textures.iter().chain(m.vs_textures.iter()) {
                renderer.render_data.load_texture(t.texture);
            }
        }
    }

    for s in to_load_samplers {
        let sampler_header_ref = package_manager()
            .get_entry(s.hash32().unwrap())
            .unwrap()
            .reference;
        let sampler_data = package_manager().read_tag(sampler_header_ref).unwrap();

        let sampler = unsafe { dcs.device.CreateSamplerState(sampler_data.as_ptr() as _) };

        if let Ok(sampler) = sampler {
            sampler_map.insert(s.key(), sampler);
        }
    }

    info!("Loaded {} samplers", sampler_map.len());

    {
        let renderer = renderer.read();
        let mut data = renderer.render_data.data_mut();
        data.materials = material_map;
        data.vshaders = vshader_map;
        data.pshaders = pshader_map;
        data.samplers = sampler_map;
    };

    Ok(LoadMapsData {
        maps,
        entity_renderers,
        placement_renderers,
        terrain_renderers,
    })
}

fn is_physics_entity(entity: ExtendedHash) -> bool {
    if let Some(nh) = entity.hash32() {
        let Ok(header) = package_manager().read_tag_struct::<Unk80809c0f>(nh) else {
            return false;
        };

        for e in &header.entity_resources {
            if e.unk0.unk10.resource_type == 0x8080916a {
                return true;
            }
        }
    }

    false
}

pub struct LoadMapsData {
    pub maps: Vec<(TagHash, MapData)>,
    pub entity_renderers: IntMap<u64, EntityRenderer>,
    pub placement_renderers: IntMap<u32, (Unk8080966d, Vec<InstancedRenderer>)>,
    pub terrain_renderers: IntMap<u32, TerrainRenderer>,
}
