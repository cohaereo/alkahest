// ! Temporary file to mitigate performance issues in some IDEs while I figure out loading routines

use std::{
    collections::HashSet,
    io::{Cursor, Read, Seek, SeekFrom},
    sync::Arc,
};

use crate::{
    activity::{SActivity, SEntityResource, Unk80808cef, Unk80808e89, Unk808092d8},
    ecs::{
        components::{
            ActivityGroup, CubemapVolume, EntityWorldId, Label, PointLight, ResourceOriginType,
            ResourcePoint, StaticInstances, Terrain, Water,
        },
        transform::{OriginalTransform, Transform},
        Scene,
    },
    entity::{Unk8080906b, Unk80809905},
    map::SMapDataTable,
    map::{
        SMeshInstanceOcclusionBounds, SShadowingLight, SimpleLight, Unk808068d4, Unk80806c98,
        Unk80806d19, Unk808085c2, Unk80808cb7, Unk80809121, Unk80809178, Unk8080917b, Unk80809802,
    },
    render::{cbuffer::ConstantBufferCached, debug::CustomDebugShape, renderer::RendererShared},
    types::{FnvHash, ResourceHash},
    util::fnv1,
};
use anyhow::Context;
use binrw::{BinReaderExt, VecArgs};
use destiny_pkg::{TagHash, TagHash64};
use glam::{Mat4, Quat, Vec3, Vec4, Vec4Swizzles};
use itertools::{multizip, Itertools};
use nohash_hasher::{IntMap, IntSet};
use windows::Win32::Graphics::{
    Direct3D::WKPDID_D3DDebugObjectName,
    Direct3D11::{ID3D11PixelShader, ID3D11SamplerState, ID3D11VertexShader},
};

use crate::structure::ExtendedHash;
use crate::{
    dxbc::{get_input_signature, get_output_signature, DxbcHeader, DxbcInputType},
    entity::{SEntityModel, Unk808072c5, Unk80809c0f},
    map::{
        MapData, SBubbleParent, SLightCollection, STerrain, Unk80806aa7, Unk80806b7f, Unk80806e68,
        Unk80806ef4, Unk8080714b,
    },
    map_resources::MapResource,
    packages::package_manager,
    render::{
        scopes::ScopeRigidModel, vertex_layout::InputElement, DeviceContextSwapchain,
        EntityRenderer, InstancedRenderer, StaticModel, TerrainRenderer,
    },
    statics::SStaticMesh,
    structure::{TablePointer, Tag},
    technique::Technique,
    types::AABB,
};

pub async fn load_maps(
    dcs: Arc<DeviceContextSwapchain>,
    renderer: RendererShared,
    map_hashes: Vec<TagHash>,
    stringmap: Arc<IntMap<u32, String>>,
    activity_hash: Option<TagHash>,
    load_ambient_activity: bool,
) -> anyhow::Result<LoadMapsData> {
    let mut vshader_map: IntMap<TagHash, (ID3D11VertexShader, Vec<InputElement>, Vec<u8>)> =
        Default::default();
    let mut pshader_map: IntMap<TagHash, (ID3D11PixelShader, Vec<InputElement>)> =
        Default::default();
    let mut sampler_map: IntMap<u64, ID3D11SamplerState> = Default::default();

    let mut maps: Vec<(TagHash, Option<TagHash64>, MapData)> = vec![];
    let mut material_map: IntMap<TagHash, Technique> = Default::default();
    let mut to_load_entitymodels: IntSet<TagHash> = Default::default();
    let renderer_ch = renderer.clone();

    let mut activity_entref_tables: IntMap<TagHash, Vec<(Tag<Unk80808e89>, ResourceHash)>> =
        Default::default();
    if let Some(activity_hash) = activity_hash {
        let activity: SActivity = package_manager().read_tag_struct(activity_hash)?;
        for u1 in &activity.unk50 {
            for map in &u1.map_references {
                let map32 = match map.hash32() {
                    Some(m) => m,
                    None => {
                        error!("Couldn't translate map hash64 {map:?}");
                        continue;
                    }
                };

                for u2 in &u1.unk18 {
                    activity_entref_tables
                        .entry(map32)
                        .or_default()
                        .push((u2.unk_entity_reference.clone(), u2.activity_phase_name2));
                }
            }
        }

        if load_ambient_activity {
            match package_manager().read_tag_struct::<SActivity>(
                activity.ambient_activity.hash32().unwrap_or_default(),
            ) {
                Ok(activity) => {
                    for u1 in &activity.unk50 {
                        for map in &u1.map_references {
                            let map32 = match map.hash32() {
                                Some(m) => m,
                                None => {
                                    error!("Couldn't translate map hash64 {map:?}");
                                    continue;
                                }
                            };

                            for u2 in &u1.unk18 {
                                activity_entref_tables.entry(map32).or_default().push((
                                    u2.unk_entity_reference.clone(),
                                    u2.activity_phase_name2,
                                ));
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

    for hash in map_hashes {
        let _span = debug_span!("Load map", %hash).entered();
        let Ok(think) = package_manager().read_tag_struct::<SBubbleParent>(hash) else {
            error!("Failed to load map {hash}");
            continue;
        };

        let mut scene = Scene::new();

        let mut unknown_root_resources: IntMap<u32, usize> = Default::default();

        let mut entity_worldid_name_map: IntMap<u64, String> = Default::default();
        if let Some(activity_entrefs) = activity_entref_tables.get(&hash) {
            for (e, _) in activity_entrefs {
                for resource in &e.unk18.entity_resources {
                    if let Some(strings) = get_entity_labels(resource.entity_resource) {
                        entity_worldid_name_map.extend(strings);
                    }
                }
            }
        }

        for map_container in &think.child_map.map_resources {
            for table in &map_container.data_tables {
                let table_data = package_manager().read_tag(table.tag()).unwrap();
                let mut cur = Cursor::new(&table_data);

                load_datatable_into_scene(
                    table,
                    table.tag(),
                    &mut cur,
                    &mut scene,
                    renderer_ch.clone(),
                    ResourceOriginType::Map,
                    0,
                    stringmap.clone(),
                    &entity_worldid_name_map,
                    &mut material_map,
                    &mut to_load_entitymodels,
                    &mut unknown_root_resources,
                )?;
            }
        }

        if let Some(activity_entrefs) = activity_entref_tables.get(&hash) {
            let mut unknown_res_types: IntSet<u32> = Default::default();
            for (e, phase_name2) in activity_entrefs {
                for resource in &e.unk18.entity_resources {
                    if resource.entity_resource.is_some() {
                        let data = package_manager().read_tag(resource.entity_resource)?;
                        let mut cur = Cursor::new(&data);
                        let res: SEntityResource = cur.read_le()?;

                        let mut data_tables = IntSet::default();
                        match res.unk18.resource_type {
                            0x808092d8 => {
                                cur.seek(SeekFrom::Start(res.unk18.offset))?;
                                let tag: Unk808092d8 = cur.read_le()?;
                                if tag.unk84.is_some() {
                                    data_tables.insert(tag.unk84);
                                }
                            }
                            0x80808cef => {
                                cur.seek(SeekFrom::Start(res.unk18.offset))?;
                                let tag: Unk80808cef = cur.read_le()?;
                                if tag.unk58.is_some() {
                                    data_tables.insert(tag.unk58);
                                }
                            }
                            u => {
                                if !unknown_res_types.contains(&u) {
                                    warn!(
                                        "Unknown activity entref resource table resource type 0x{u:x}"
                                    );

                                    unknown_res_types.insert(u);
                                }
                            }
                        }

                        let mut data_tables2 = IntSet::default();
                        // TODO(cohae): This is a very dirty hack to find every other data table in the entityresource. We need to fully flesh out the EntityResource format first.
                        // TODO(cohae): PS: gets assigned as Activity2 to keep them separate from known tables
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
                            warn!("TODO: Found {} map data tables ({}) EntityResource by brute force ({} found normally)", data_tables2.len(), tstr, data_tables.len());
                        }

                        for table_tag in data_tables {
                            let data = package_manager().read_tag(table_tag)?;
                            let mut cur = Cursor::new(&data);
                            let table: SMapDataTable = cur.read_le()?;

                            load_datatable_into_scene(
                                &table,
                                table_tag,
                                &mut cur,
                                &mut scene,
                                renderer_ch.clone(),
                                ResourceOriginType::Activity,
                                phase_name2.0,
                                stringmap.clone(),
                                &entity_worldid_name_map,
                                &mut material_map,
                                &mut to_load_entitymodels,
                                &mut unknown_root_resources,
                            )?;
                        }

                        for table_tag in data_tables2 {
                            let data = package_manager().read_tag(table_tag)?;
                            let mut cur = Cursor::new(&data);
                            let table: SMapDataTable = cur.read_le()?;

                            load_datatable_into_scene(
                                &table,
                                table_tag,
                                &mut cur,
                                &mut scene,
                                renderer_ch.clone(),
                                ResourceOriginType::Activity2,
                                phase_name2.0,
                                stringmap.clone(),
                                &entity_worldid_name_map,
                                &mut material_map,
                                &mut to_load_entitymodels,
                                &mut unknown_root_resources,
                            )?;
                        }
                    } else {
                        warn!("null entity resource tag in {}", resource.tag());
                    }
                }
            }
        }

        for (rtype, count) in unknown_root_resources.into_iter() {
            warn!("World origin resource {} is not parsed! Resource points might be missing ({} instances)", TagHash(rtype), count);
        }

        let map_name = stringmap
            .get(&think.map_name.0)
            .cloned()
            .unwrap_or(format!("[MissingString_{:08x}]", think.map_name.0));
        let hash64 = package_manager()
            .hash64_table
            .iter()
            .find(|v| v.1.hash32 == hash)
            .map(|v| TagHash64(*v.0));

        info!(
            "Map {:x?} '{map_name}' - {} instance groups, {} decals",
            think.map_name,
            scene.query::<&StaticInstances>().iter().count(),
            scene
                .query::<&ResourcePoint>()
                .iter()
                .filter(|(_, r)| r.resource.is_decal())
                .count()
        );

        let mut point_lights = vec![
            SimpleLight {
                pos: Vec4::ZERO,
                attenuation: Vec4::ONE,
            };
            2
        ];
        for (_, (transform, light)) in scene.query::<(&Transform, &PointLight)>().iter() {
            point_lights.push(SimpleLight {
                pos: transform.translation.extend(1.0),
                attenuation: light.attenuation,
            });
        }

        maps.push((
            hash,
            hash64,
            MapData {
                hash,
                name: map_name,
                scene,
            },
        ));
    }

    let to_load_entities: HashSet<ExtendedHash> = maps
        .iter_mut()
        .flat_map(|(_, _, v)| {
            v.scene
                .query::<&ResourcePoint>()
                .iter()
                .map(|(_, r)| r.entity)
                .collect_vec()
        })
        .filter(|v| v.is_some())
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
                        let model: Tag<SEntityModel> = cur.read_le()?;
                        cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x3c0))?;
                        let entity_material_map: TablePointer<Unk808072c5> = cur.read_le()?;
                        cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x400))?;
                        let materials: TablePointer<TagHash> = cur.read_le()?;

                        for m in &materials {
                            if let Ok(mat) = package_manager().read_tag_struct(*m) {
                                material_map.insert(*m, Technique::load(&renderer, mat, *m, true));
                            }
                        }

                        for m in &model.meshes {
                            for p in &m.parts {
                                if p.material.is_some() {
                                    material_map.insert(
                                        p.material,
                                        Technique::load(
                                            &renderer,
                                            package_manager().read_tag_struct(p.material)?,
                                            p.material,
                                            true,
                                        ),
                                    );
                                }
                            }
                        }

                        match debug_span!("load EntityRenderer").in_scope(|| {
                            EntityRenderer::load(
                                model.0,
                                entity_material_map.to_vec(),
                                materials.to_vec(),
                                &renderer,
                            )
                        }) {
                            Ok(er) => {
                                entity_renderers.insert(te.key(), er);
                            }
                            Err(e) => {
                                error!("Failed to load entity {te:?}: {e}");
                            }
                        }

                        // println!(" - EntityModel {model:?}");
                    }
                    u => {
                        debug!(
                            "\t- Unknown entity resource type {:08X}/{:08X} (table {})",
                            u.to_be(),
                            e.unk0.unk10.resource_type.to_be(),
                            e.unk0.tag()
                        )
                    }
                }
            }
        }
    }

    info!("Loading {} background entities", to_load_entitymodels.len());

    for t in to_load_entitymodels {
        let renderer = renderer.read();
        let model: SEntityModel = package_manager().read_tag_struct(t)?;

        for m in &model.meshes {
            for p in &m.parts {
                if p.material.is_some() {
                    material_map.insert(
                        p.material,
                        Technique::load(
                            &renderer,
                            package_manager().read_tag_struct(p.material)?,
                            p.material,
                            true,
                        ),
                    );
                }
            }
        }

        match debug_span!("load EntityRenderer")
            .in_scope(|| EntityRenderer::load(model, vec![], vec![], &renderer))
        {
            Ok(er) => {
                entity_renderers.insert(t.0 as u64, er);
            }
            Err(e) => {
                error!("Failed to load entity {t}: {e}");
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
    for (_, _, m) in &mut maps {
        for (_, (transform, rp)) in m.scene.query_mut::<(&Transform, &mut ResourcePoint)>() {
            if let Some(ent) = entity_renderers.get(&rp.entity_key()) {
                let mm = transform.to_mat4();

                let model_matrix = Mat4::from_cols(
                    mm.x_axis.truncate().extend(mm.w_axis.x),
                    mm.y_axis.truncate().extend(mm.w_axis.y),
                    mm.z_axis.truncate().extend(mm.w_axis.z),
                    mm.w_axis,
                );

                let alt_matrix = Mat4::from_cols(
                    Vec3::ONE.extend(mm.w_axis.x),
                    Vec3::ONE.extend(mm.w_axis.y),
                    Vec3::ONE.extend(mm.w_axis.z),
                    Vec4::W,
                );

                rp.entity_cbuffer = ConstantBufferCached::create_init(
                    dcs.clone(),
                    &ScopeRigidModel {
                        mesh_to_world: model_matrix,
                        position_scale: ent.mesh_scale(),
                        position_offset: ent.mesh_offset(),
                        texcoord0_scale_offset: ent.texcoord_transform(),
                        dynamic_sh_ao_values: Vec4::new(1.0, 1.0, 1.0, 0.0),
                        unk8: [alt_matrix; 8],
                    },
                )
                .unwrap();
            }
        }
    }

    let mut to_load_samplers: HashSet<ExtendedHash> = Default::default();

    info_span!("Loading shaders").in_scope(|| {
        for (t, m) in material_map.iter() {
            // TODO(cohae): Technique is responsible for loading samplers
            for stage in m.all_stages() {
                for sampler in stage.shader.samplers.iter() {
                    to_load_samplers.insert(*sampler);
                }
            }

            if let Some(v) = package_manager().get_entry(m.stage_vertex.shader.shader) {
                let _span = debug_span!("load vshader", shader = ?m.stage_vertex.shader).entered();

                vshader_map
                    .entry(m.stage_vertex.shader.shader)
                    .or_insert_with(|| {
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

                            let name =
                                format!("VS {:?} (mat {})\0", m.stage_vertex.shader.shader, t);
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

            if let Some(v) = package_manager().get_entry(m.stage_pixel.shader.shader) {
                let _span =
                    debug_span!("load pshader", shader = ?m.stage_pixel.shader.shader).entered();

                pshader_map
                    .entry(m.stage_pixel.shader.shader)
                    .or_insert_with(|| {
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

                            let name =
                                format!("PS {:?} (mat {})\0", m.stage_pixel.shader.shader, t);
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
        // TODO(cohae): Technique is responsible for loading textures
        let renderer = renderer.read();
        for m in material_map.values() {
            for stage in m.all_stages() {
                for t in stage.shader.textures.iter() {
                    renderer.render_data.load_texture(t.texture);
                }
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
        data.techniques.extend(material_map);
        data.vshaders.extend(vshader_map);
        data.pshaders.extend(pshader_map);
        data.samplers.extend(sampler_map);
    };

    #[cfg(not(feature = "keep_map_order"))]
    maps.sort_by_key(|m| m.2.name.clone());

    Ok(LoadMapsData {
        maps,
        entity_renderers,
    })
}

pub struct LoadMapsData {
    pub maps: Vec<(TagHash, Option<TagHash64>, MapData)>,
    pub entity_renderers: IntMap<u64, EntityRenderer>,
}

// clippy: asset system will fix this lint on it's own (i hope)
#[allow(clippy::too_many_arguments)]
fn load_datatable_into_scene<R: Read + Seek>(
    table: &SMapDataTable,
    table_hash: TagHash,
    table_data: &mut R,
    scene: &mut Scene,
    renderer: RendererShared,
    resource_origin: ResourceOriginType,
    group_id: u32,
    stringmap: Arc<IntMap<u32, String>>,
    entity_worldid_name_map: &IntMap<u64, String>,

    material_map: &mut IntMap<TagHash, Technique>,
    to_load_entitymodels: &mut IntSet<TagHash>,
    unknown_root_resources: &mut IntMap<u32, usize>,
) -> anyhow::Result<()> {
    let renderer = renderer.read();
    let dcs = renderer.dcs.clone();

    let mut ents = vec![];
    for data in &table.data_entries {
        let transform = Transform {
            translation: Vec3::new(data.translation.x, data.translation.y, data.translation.z),
            rotation: data.rotation.into(),
            scale: Vec3::splat(data.translation.w),
        };

        let base_rp = ResourcePoint {
            entity: data.entity,
            has_havok_data: is_physics_entity(data.entity),
            origin: resource_origin,
            resource_type: data.data_resource.resource_type,
            resource: MapResource::Unknown(
                data.data_resource.resource_type,
                data.world_id,
                data.entity,
                data.data_resource,
                table_hash,
            ),
            entity_cbuffer: ConstantBufferCached::create_empty(dcs.clone())?,
        };

        if data.data_resource.is_valid {
            match data.data_resource.resource_type {
                // D2Class_C96C8080 (placement)
                0x80806cc9 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset + 16))
                        .unwrap();
                    let preheader_tag: TagHash = table_data.read_le().unwrap();
                    let preheader: Unk80806ef4 =
                        package_manager().read_tag_struct(preheader_tag).unwrap();

                    info_span!("Loading static instances").in_scope(|| {
                        'next_instance: for s in &preheader.instances.instance_groups {
                            let mesh_tag = preheader.instances.statics[s.static_index as usize];
                            let mheader: SStaticMesh = debug_span!("load tag Unk808071a7")
                                .in_scope(|| package_manager().read_tag_struct(mesh_tag).unwrap());
                            for m in &mheader.materials {
                                if m.is_some()
                                    && !material_map.contains_key(m)
                                    && !renderer.render_data.data().techniques.contains_key(m)
                                {
                                    material_map.insert(
                                        *m,
                                        Technique::load(
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
                                if m.is_some()
                                    && !material_map.contains_key(&m)
                                    && !renderer.render_data.data().techniques.contains_key(&m)
                                {
                                    material_map.insert(
                                        m,
                                        Technique::load(
                                            &renderer,
                                            package_manager().read_tag_struct(m).unwrap(),
                                            m,
                                            true,
                                        ),
                                    );
                                }
                            }

                            match StaticModel::load(mheader, &renderer) {
                                Ok(model) => {
                                    let transforms =
                                        &preheader.instances.transforms[s.instance_start as usize
                                            ..(s.instance_start + s.instance_count) as usize];

                                    let bounds = if ((s.instance_start + s.instance_count) as usize)
                                        <= preheader.instances.occlusion_bounds.bounds.len()
                                    {
                                        preheader.instances.occlusion_bounds.bounds[s.instance_start
                                            as usize
                                            ..(s.instance_start + s.instance_count) as usize]
                                            .to_vec()
                                    } else {
                                        warn!("Instance group doesn't have enough occlusion bounds, need range {}..{}, but there are only {} bounds", s.instance_start, s.instance_start + s.instance_count, preheader.instances.occlusion_bounds.bounds.len());
                                        vec![
                                            SMeshInstanceOcclusionBounds {
                                                bb: AABB::INFINITE,
                                                unk20: [0; 4]
                                            };
                                            s.instance_count as usize
                                        ]
                                    };

                                    let instanced_renderer = match InstancedRenderer::load(
                                        Arc::new(model),
                                        transforms,
                                        &bounds,
                                        dcs.clone(),
                                    ) {
                                        Ok(o) => o,
                                        Err(e) => {
                                            error!("Failed to create InstancedRenderer: {e}");
                                            continue 'next_instance;
                                        }
                                    };

                                    ents.push(scene.spawn((
                                        StaticInstances(instanced_renderer, mesh_tag),
                                        EntityWorldId(data.world_id),
                                    )));
                                }
                                Err(e) => {
                                    error!(model = ?mesh_tag, "Failed to load model: {e}");
                                }
                            }
                        }
                    });
                }
                // D2Class_7D6C8080 (terrain)
                0x80806c7d => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset))
                        .unwrap();

                    let terrain_resource: Unk8080714b = table_data.read_le().unwrap();
                    let terrain: STerrain = package_manager()
                        .read_tag_struct(terrain_resource.terrain)
                        .unwrap();

                    for p in &terrain.mesh_parts {
                        if p.material.is_some() {
                            material_map.insert(
                                p.material,
                                Technique::load(
                                    &renderer,
                                    package_manager().read_tag_struct(p.material)?,
                                    p.material,
                                    true,
                                ),
                            );
                        }
                    }

                    for t in &terrain.mesh_groups {
                        renderer
                            .render_data
                            .load_texture(ExtendedHash::Hash32(t.dyemap));
                    }

                    match TerrainRenderer::load(terrain, dcs.clone(), &renderer) {
                        Ok(r) => {
                            ents.push(scene.spawn((Terrain(r), EntityWorldId(data.world_id))));
                        }
                        Err(e) => {
                            error!("Failed to load terrain: {e}");
                        }
                    }
                }
                // Cubemap volume
                0x80806695 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset))
                        .unwrap();

                    let cubemap_volume: Unk80806b7f = table_data.read_le().unwrap();
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

                    let volume_min = extents_center - extents;
                    let volume_max = extents_center + extents;

                    renderer
                        .render_data
                        .load_texture(ExtendedHash::Hash32(cubemap_volume.cubemap_texture));

                    let aabb = AABB {
                        min: volume_min.truncate().into(),
                        max: volume_max.truncate().into(),
                    };
                    ents.push(scene.spawn((
                        Transform {
                            translation: extents_center.xyz(),
                            rotation: transform.rotation,
                            ..Default::default()
                        },
                        CubemapVolume(
                            cubemap_volume.cubemap_texture,
                            aabb,
                            cubemap_volume.cubemap_name.to_string(),
                        ),
                        ResourcePoint {
                            resource: MapResource::CubemapVolume(Box::new(cubemap_volume), aabb),
                            ..base_rp
                        },
                        EntityWorldId(data.world_id),
                    )));
                }
                0x808067b5 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset + 16))
                        .unwrap();
                    let tag: TagHash = table_data.read_le().unwrap();

                    ents.push(scene.spawn((
                        transform,
                        ResourcePoint {
                            resource: MapResource::Unk808067b5(tag),
                            ..base_rp
                        },
                        EntityWorldId(data.world_id),
                    )));
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

                    let header: Unk80806e68 = package_manager().read_tag_struct(tag).unwrap();

                    for inst in &header.instances {
                        for i in inst.start..(inst.start + inst.count) {
                            let transform = header.transforms[i as usize];
                            let bounds = &header.occlusion_bounds.bounds[i as usize];
                            ents.push(scene.spawn((
                                Transform {
                                    translation: Vec3::new(transform.x, transform.y, transform.z),
                                    ..Default::default()
                                },
                                ResourcePoint {
                                    resource: MapResource::Decal {
                                        material: inst.material,
                                        bounds: bounds.bb,
                                        scale: transform.w,
                                    },
                                    entity_cbuffer: ConstantBufferCached::create_empty(
                                        dcs.clone(),
                                    )?,
                                    ..base_rp
                                },
                                EntityWorldId(data.world_id),
                            )));
                        }
                    }
                }
                // (ambient) sound source
                0x8080666f => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset + 16))
                        .unwrap();
                    let tag: ExtendedHash = table_data.read_le().unwrap();
                    if !tag.is_some() || tag.hash32().is_none() {
                        // TODO: should be handled a bit more gracefully, shouldnt drop the whole node
                        // TODO: do the same for other resources ^
                        continue;
                    }

                    let header = package_manager()
                        .read_tag_struct::<Unk80809802>(tag.hash32().unwrap())
                        .ok();

                    ents.push(scene.spawn((
                        transform,
                        ResourcePoint {
                            resource: MapResource::AmbientSound(header),
                            ..base_rp
                        },
                        EntityWorldId(data.world_id),
                    )));
                }
                0x80806aa3 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset + 16))
                        .unwrap();
                    let tag: TagHash = table_data.read_le().unwrap();
                    if tag.is_none() {
                        continue;
                    }

                    let header: Unk80806aa7 = package_manager().read_tag_struct(tag).unwrap();

                    for (unk8, unk18, _unk28) in itertools::multizip((
                        header.unk8.iter(),
                        header.unk18.iter(),
                        header.unk28.iter(),
                    )) {
                        to_load_entitymodels.insert(unk8.unk60.entity_model);

                        let mat = Mat4 {
                            x_axis: unk8.transform[0].into(),
                            y_axis: unk8.transform[1].into(),
                            z_axis: unk8.transform[2].into(),
                            w_axis: unk8.transform[3].into(),
                        };

                        ents.push(scene.spawn((
                            Transform::from_mat4(mat),
                            ResourcePoint {
                                resource: MapResource::Unk80806aa3(
                                    unk18.bb,
                                    unk8.unk60.entity_model,
                                    mat,
                                ),
                                entity_cbuffer: ConstantBufferCached::create_empty(dcs.clone())?,
                                ..base_rp
                            },
                            EntityWorldId(data.world_id),
                        )));
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

                    let header: SLightCollection = package_manager().read_tag_struct(tag).unwrap();

                    for (i, (transform, light, bounds)) in multizip((
                        &header.unk40,
                        &header.unk30,
                        &header.occlusion_bounds.bounds,
                    ))
                    .enumerate()
                    {
                        if light.technique_shading.is_some() {
                            material_map.insert(
                                light.technique_shading,
                                Technique::load(
                                    &renderer,
                                    package_manager().read_tag_struct(light.technique_shading)?,
                                    light.technique_shading,
                                    true,
                                ),
                            );
                        }

                        ents.push(scene.spawn((
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
                            ResourcePoint {
                                resource: MapResource::Light(bounds.bb, tag, i),

                                entity_cbuffer: ConstantBufferCached::create_empty(dcs.clone())?,
                                ..base_rp
                            },
                            EntityWorldId(data.world_id),
                            PointLight {
                                attenuation: Vec4::ONE,
                            },
                            light.clone(),
                            bounds.bb,
                        )));
                    }
                }
                0x80808cb5 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset + 16))
                        .unwrap();
                    let tag: TagHash = table_data.read_le().unwrap();
                    if !tag.is_some() {
                        continue;
                    }

                    let header: Unk80808cb7 = package_manager().read_tag_struct(tag).unwrap();

                    for transform in header.unk8.iter() {
                        ents.push(scene.spawn((
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
                            ResourcePoint {
                                resource: MapResource::RespawnPoint(transform.unk20),

                                entity_cbuffer: ConstantBufferCached::create_empty(dcs.clone())?,
                                ..base_rp
                            },
                            EntityWorldId(data.world_id),
                        )));
                    }
                }
                0x808085c0 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset + 16))
                        .unwrap();
                    let tag: TagHash = table_data.read_le().unwrap();
                    if !tag.is_some() {
                        continue;
                    }

                    let header: Unk808085c2 = package_manager().read_tag_struct(tag).unwrap();

                    for transform in header.unk8.iter() {
                        ents.push(scene.spawn((
                            Transform {
                                translation: Vec3::new(
                                    transform.translation.x,
                                    transform.translation.y,
                                    transform.translation.z,
                                ),
                                ..Default::default()
                            },
                            ResourcePoint {
                                resource: MapResource::Unk808085c0,

                                entity_cbuffer: ConstantBufferCached::create_empty(dcs.clone())?,
                                ..base_rp
                            },
                            EntityWorldId(data.world_id),
                        )));
                    }
                }
                0x8080684d => {
                    // TODO(cohae): Collection of havok files
                }
                0x80806a40 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset + 16))
                        .unwrap();
                    let tag: TagHash = table_data.read_le().unwrap();
                    if !tag.is_some() {
                        continue;
                    }

                    let header: Unk80806d19 = package_manager().read_tag_struct(tag).unwrap();

                    for transform in header.unk50.iter() {
                        ents.push(scene.spawn((
                            Transform {
                                translation: Vec3::new(
                                    transform.translation.x,
                                    transform.translation.y,
                                    transform.translation.z,
                                ),
                                ..Default::default()
                            },
                            ResourcePoint {
                                resource: MapResource::Unk80806a40,

                                entity_cbuffer: ConstantBufferCached::create_empty(dcs.clone())?,
                                ..base_rp
                            },
                            EntityWorldId(data.world_id),
                        )));
                    }
                }
                // Foliage
                0x80806cc3 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset + 16))
                        .unwrap();
                    let header_tag: TagHash = table_data.read_le().unwrap();
                    let header: Unk80806c98 =
                        package_manager().read_tag_struct(header_tag).unwrap();

                    for b in &header.unk4c.bounds {
                        ents.push(scene.spawn((
                            Transform {
                                translation: b.bb.center(),
                                ..Default::default()
                            },
                            ResourcePoint {
                                resource: MapResource::Decoration(b.bb, header_tag),

                                entity_cbuffer: ConstantBufferCached::create_empty(dcs.clone())?,
                                ..base_rp
                            },
                            EntityWorldId(data.world_id),
                        )));
                    }

                    // resource_points.push(ResourcePoint {
                    //     transform: Transform {
                    //         translation: header.bounds.center(),
                    //         ..Default::default()
                    //     },
                    //     entity: data.entity,
                    //     has_havok_data: is_physics_entity(data.entity),
                    //     world_id: data.world_id,
                    //     resource_type: data.data_resource.resource_type,
                    //     resource: MapResource::Unk80806cc3(header.bounds),
                    // });
                }
                0x80806c5e => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset + 16))
                        .unwrap();
                    let tag: TagHash = table_data.read_le().unwrap();
                    let light: SShadowingLight = package_manager().read_tag_struct(tag)?;

                    if light.technique_shading.is_some() {
                        material_map.insert(
                            light.technique_shading,
                            Technique::load(
                                &renderer,
                                package_manager().read_tag_struct(light.technique_shading)?,
                                light.technique_shading,
                                true,
                            ),
                        );
                    }

                    ents.push(scene.spawn((
                        transform,
                        ResourcePoint {
                            resource: MapResource::ShadowingLight(tag),
                            ..base_rp
                        },
                        EntityWorldId(data.world_id),
                        light,
                    )));
                }
                0x80809178 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset))
                        .unwrap();

                    let d: Unk80809178 = table_data.read_le()?;
                    let name = stringmap
                        .get(&d.area_name.0)
                        .cloned()
                        .unwrap_or_else(|| format!("[MissingString_{:08x}]", d.area_name.0));

                    ents.push(scene.spawn((
                        transform,
                        ResourcePoint {
                            resource: MapResource::NamedArea(d, name),
                            has_havok_data: true,
                            ..base_rp
                        },
                        EntityWorldId(data.world_id),
                    )));
                }
                0x8080917b => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset))
                        .unwrap();

                    let d: Unk8080917b = table_data.read_le()?;

                    let havok_debugshape =
                        if let Ok(havok_data) = package_manager().read_tag(d.unk0.havok_file) {
                            let mut cur = Cursor::new(&havok_data);
                            match destiny_havok::shape_collection::read_shape_collection(&mut cur) {
                                Ok(o) => {
                                    if (d.unk0.shape_index as usize) < o.len() {
                                        CustomDebugShape::from_havok_shape(
                                            &dcs,
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

                    ents.push(scene.spawn((
                        transform,
                        ResourcePoint {
                            resource: MapResource::KillOrTurnbackBarrier(
                                d.unk0.havok_file,
                                d.unk0.shape_index,
                                havok_debugshape,
                            ),
                            has_havok_data: true,
                            ..base_rp
                        },
                        EntityWorldId(data.world_id),
                    )));
                }
                0x80809121 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset))
                        .unwrap();

                    let d: Unk80809121 = table_data.read_le()?;

                    ents.push(scene.spawn((
                        transform,
                        ResourcePoint {
                            resource: MapResource::Unk80809121(d.havok_file),
                            has_havok_data: true,
                            ..base_rp
                        },
                        EntityWorldId(data.world_id),
                    )));
                }
                0x808068d4 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset))
                        .unwrap();

                    let d: Unk808068d4 = table_data.read_le()?;
                    to_load_entitymodels.insert(d.entity_model);

                    ents.push(scene.spawn((
                        transform,
                        ResourcePoint {
                            resource: MapResource::Unk808068d4(d.entity_model),
                            has_havok_data: true,
                            ..base_rp
                        },
                        EntityWorldId(data.world_id),
                        Water,
                    )));
                }
                u => {
                    if data.translation.x == 0.0
                        && data.translation.y == 0.0
                        && data.translation.z == 0.0
                    {
                        match unknown_root_resources.entry(u) {
                            std::collections::hash_map::Entry::Occupied(mut o) => {
                                *o.get_mut() += 1;
                            }
                            std::collections::hash_map::Entry::Vacant(v) => {
                                v.insert(1);
                            }
                        }
                        debug!("World origin resource {} is not parsed! Resource points might be missing (table {})", TagHash(u), table_hash);
                    }

                    debug!(
                        "Skipping unknown resource type {u:x} {:?} (table file {:?})",
                        data.translation, table_hash
                    );
                    ents.push(scene.spawn((transform, base_rp, EntityWorldId(data.world_id))));
                }
            };
        } else {
            ents.push(scene.spawn((
                transform,
                ResourcePoint {
                    resource: MapResource::Entity(data.entity, data.world_id),
                    ..base_rp
                },
                EntityWorldId(data.world_id),
            )));
        }
    }

    if group_id != 0 {
        for e in &ents {
            scene.insert_one(*e, ActivityGroup(group_id)).ok();
        }
    }

    for e in ents {
        if let Ok(transform) = scene.get::<&Transform>(e).map(|t| (*t)) {
            scene.insert_one(e, OriginalTransform(transform)).ok();
        };

        let Some(world_id) = scene.get::<&EntityWorldId>(e).map(|w| w.0).ok() else {
            continue;
        };

        if let Some(name) = entity_worldid_name_map.get(&world_id) {
            scene.insert_one(e, Label(name.clone())).ok();
        }
    }

    Ok(())
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

fn get_entity_labels(entity: TagHash) -> Option<IntMap<u64, String>> {
    let data: Vec<u8> = package_manager().read_tag(entity).ok()?;
    let mut cur = Cursor::new(&data);

    let e = cur.read_le::<SEntityResource>().ok()?;
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
            let count: u64 = cur.read_le().ok()?;
            cur.seek(SeekFrom::Start(offset + 8)).ok()?;
            world_id_list = cur
                .read_le_args(VecArgs {
                    count: count as usize,
                    inner: (),
                })
                .ok()?;
            // let list: TablePointer<Unk80809905> = cur.read_le().ok()?;
            // world_id_list = list.take_data();
            break;
        }
    }

    // if matches!(e.unk18.resource_type, 0x80808cf8 | 0x808098fa) {
    //     cur.seek(SeekFrom::Start(e.unk18.offset + 0x50)).ok()?;
    //     let list: TablePointer<Unk80809905> = cur.read_le().ok()?;
    //     world_id_list = list.take_data();
    // }

    // TODO(cohae): There's volumes and stuff without a world ID that still have a name
    world_id_list.retain(|w| w.world_id != u64::MAX);

    let mut name_hash_map: IntMap<FnvHash, String> = IntMap::default();

    let tablethingy: Unk8080906b = package_manager().read_tag_struct(e.unk80).ok()?;
    for v in tablethingy.unk0.into_iter() {
        if let Some(name_ptr) = &v.unk0_name_pointer {
            name_hash_map.insert(fnv1(&name_ptr.name), name_ptr.name.to_string());
        }
    }

    Some(
        world_id_list
            .into_iter()
            .filter_map(|w| Some((w.world_id, name_hash_map.get(&w.name_hash)?.clone())))
            .collect(),
    )
}
