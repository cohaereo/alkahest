// ! Temporary file to mitigate performance issues in some IDEs while I figure out loading routines

use std::{
    collections::HashSet,
    io::{Cursor, Read, Seek, SeekFrom},
    sync::Arc,
};

use alkahest_data::{
    activity::{SActivity, SDestination, SEntityResource, Unk80808cef, Unk80808e89, Unk808092d8},
    common::ResourceHash,
    entity::{SEntityModel, Unk808072c5, Unk8080906b, Unk80809905, Unk80809c0f},
    map::{
        SBubbleParent, SBubbleParentShallow, SLightCollection, SMapDataTable, SShadowingLight,
        SSlipSurfaceVolume, STerrain, Unk808068d4, Unk80806aa7, Unk80806ac2, Unk80806b7f,
        Unk80806c98, Unk80806d19, Unk80806e68, Unk80806ef4, Unk8080714b, Unk80808246, Unk808085c2,
        Unk80808604, Unk80808cb7, Unk80809178, Unk8080917b, Unk80809802,
    },
    occlusion::{SObjectOcclusionBounds, AABB},
    statics::SStaticMesh,
    ExtendedHash, Tag,
};
use anyhow::Context;
use binrw::BinReaderExt;
use destiny_pkg::TagHash;
use glam::{Mat4, Quat, Vec3, Vec4, Vec4Swizzles};
use itertools::{multizip, Itertools};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::{FxHashMap, FxHashSet};
use tiger_parse::{dpkg::PackageManagerExt, Endian, FnvHash, TigerReadable};
use windows::Win32::Graphics::{
    Direct3D::WKPDID_D3DDebugObjectName,
    Direct3D11::{ID3D11PixelShader, ID3D11SamplerState, ID3D11VertexShader},
};

use crate::{
    dxbc::{get_input_signature, get_output_signature, DxbcHeader, DxbcInputType},
    ecs::{
        components::{
            ActivityGroup, CubemapVolume, EntityWorldId, Label, Light, PointLight,
            ResourceOriginType, ResourcePoint, StaticInstances, Terrain, Water,
        },
        tags::{insert_tag, EntityTag},
        transform::{OriginalTransform, Transform},
        Scene,
    },
    map_resources::MapResource,
    packages::package_manager,
    render::{
        cbuffer::ConstantBufferCached, debug::CustomDebugShape, renderer::RendererShared,
        scopes::ScopeRigidModel, vertex_layout::InputElement, DeviceContextSwapchain,
        EntityRenderer, InstancedRenderer, StaticModel, TerrainRenderer,
    },
    technique::Technique,
    text::StringContainer,
    util::fnv1,
};

pub fn get_map_name(
    map_hash: TagHash,
    stringmap: &FxHashMap<u32, String>,
) -> anyhow::Result<String> {
    let _span = info_span!("Get map name", %map_hash).entered();
    let map_name = match package_manager().read_tag_struct::<SBubbleParentShallow>(map_hash) {
        Ok(m) => m.map_name,
        Err(e) => {
            anyhow::bail!("Failed to load map {map_hash}: {e}");
        }
    };

    Ok(stringmap
        .get(&map_name.0)
        .cloned()
        .unwrap_or(format!("[MissingString_{:08x}]", map_name.0)))
}

pub fn query_activity_maps(
    activity_hash: TagHash,
    stringmap: &FxHashMap<u32, String>,
) -> anyhow::Result<Vec<(TagHash, String)>> {
    let _span = info_span!("Query activity maps").entered();
    let activity: SActivity = package_manager().read_tag_struct(activity_hash)?;
    let mut string_container = StringContainer::default();
    if let Ok(destination) = package_manager().read_tag_struct::<SDestination>(activity.destination)
    {
        if let Ok(sc) = StringContainer::load(destination.string_container) {
            string_container = sc;
        }
    }

    let mut maps = vec![];
    for u1 in &activity.unk50 {
        for map in &u1.map_references {
            let map_name = match package_manager().read_tag_struct::<SBubbleParentShallow>(*map) {
                Ok(m) => m.map_name,
                Err(e) => {
                    error!("Failed to load map {map}: {e}");
                    continue;
                }
            };

            let map_name = string_container
                .get(&map_name.0)
                .cloned()
                .unwrap_or_else(|| {
                    // Fall back to global stringmap
                    stringmap
                        .get(&map_name.0)
                        .cloned()
                        .unwrap_or(format!("[MissingString_{:08x}]", map_name.0))
                });

            maps.push((map.hash32(), map_name));
        }
    }

    Ok(maps)
}

pub async fn load_map_scene(
    dcs: Arc<DeviceContextSwapchain>,
    renderer: RendererShared,
    map_hash: TagHash,
    stringmap: Arc<FxHashMap<u32, String>>,
    activity_hash: Option<TagHash>,
    load_ambient_activity: bool,
) -> anyhow::Result<LoadMapData> {
    let mut vshader_map: FxHashMap<TagHash, (ID3D11VertexShader, Vec<InputElement>, Vec<u8>)> =
        Default::default();
    let mut pshader_map: FxHashMap<TagHash, (ID3D11PixelShader, Vec<InputElement>)> =
        Default::default();
    let mut sampler_map: FxHashMap<u64, ID3D11SamplerState> = Default::default();

    let mut material_map: FxHashMap<TagHash, Technique> = Default::default();
    let mut to_load_entitymodels: FxHashSet<TagHash> = Default::default();
    let renderer_ch = renderer.clone();

    let _span = debug_span!("Load map", %map_hash).entered();
    let Ok(bubble_parent) = package_manager().read_tag_struct::<SBubbleParent>(map_hash) else {
        anyhow::bail!("Failed to load map {map_hash}");
    };

    let mut activity_entrefs: Vec<(Tag<Unk80808e89>, ResourceHash, ResourceOriginType)> =
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
                        ResourceOriginType::Activity,
                    ));
                }
            }
        }

        if load_ambient_activity {
            match package_manager().read_tag_struct::<SActivity>(activity.ambient_activity) {
                Ok(activity) => {
                    for u1 in &activity.unk50 {
                        for map in &u1.map_references {
                            if map.hash32() != map_hash {
                                continue;
                            }

                            for u2 in &u1.unk18 {
                                activity_entrefs.push((
                                    u2.unk_entity_reference.clone(),
                                    u2.activity_phase_name2,
                                    ResourceOriginType::Ambient,
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

    let mut scene = Scene::new();

    let mut unknown_root_resources: FxHashMap<u32, Vec<TagHash>> = Default::default();

    let mut entity_worldid_name_map: FxHashMap<u64, String> = Default::default();
    for (e, _, _) in &activity_entrefs {
        for resource in &e.unk18.entity_resources {
            if let Some(strings) = get_entity_labels(resource.entity_resource) {
                entity_worldid_name_map.extend(strings);
            }
        }
    }

    for map_container in &bubble_parent.child_map.map_resources {
        for table in &map_container.data_tables {
            let table_data = package_manager().read_tag(table.hash()).unwrap();
            let mut cur = Cursor::new(&table_data);

            load_datatable_into_scene(
                table,
                table.hash(),
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

    let mut unknown_res_types: FxHashSet<u32> = Default::default();
    for (e, phase_name2, origin) in activity_entrefs {
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
                        if !unknown_res_types.contains(&u) {
                            warn!(
                                        "Unknown activity entref resource table resource type 0x{u:x} in resource table {}", resource.entity_resource
                                    );

                            unknown_res_types.insert(u);
                        }
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
                    warn!("TODO: Found {} map data tables ({}) EntityResource by brute force ({} found normally)", data_tables2.len(), tstr, data_tables.len());
                }

                for table_tag in data_tables {
                    let data = package_manager().read_tag(table_tag)?;
                    let mut cur = Cursor::new(&data);
                    let table: SMapDataTable =
                        TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                    load_datatable_into_scene(
                        &table,
                        table_tag,
                        &mut cur,
                        &mut scene,
                        renderer_ch.clone(),
                        origin,
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
                    let table: SMapDataTable =
                        TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                    load_datatable_into_scene(
                        &table,
                        table_tag,
                        &mut cur,
                        &mut scene,
                        renderer_ch.clone(),
                        // cohae: yes, this means bruteforced ambient data tables will always be shown as ambient, but i don't think it matters once we fix the normal bruteforced activity tables
                        if origin == ResourceOriginType::Ambient {
                            origin
                        } else {
                            ResourceOriginType::ActivityBruteforce
                        },
                        phase_name2.0,
                        stringmap.clone(),
                        &entity_worldid_name_map,
                        &mut material_map,
                        &mut to_load_entitymodels,
                        &mut unknown_root_resources,
                    )?;
                }
            } else {
                warn!("null entity resource tag in {}", resource.hash());
            }
        }
    }

    for (rtype, tables) in unknown_root_resources.into_iter() {
        warn!("World origin resource {} is not parsed! Resource points might be missing (found in these tables [{}])", TagHash(rtype), tables.iter().map(|v| v.to_string()).join(", "));
    }

    info!(
        "Map {:x?} '{}' - {} instance groups, {} decals",
        bubble_parent.map_name,
        stringmap
            .get(&bubble_parent.map_name.0)
            .cloned()
            .unwrap_or_else(|| format!("[MissingString_{:08x}]", bubble_parent.map_name.0)),
        scene.query::<&StaticInstances>().iter().count(),
        scene
            .query::<&ResourcePoint>()
            .iter()
            .filter(|(_, r)| r.resource.is_decal())
            .count()
    );

    let to_load_entities: HashSet<ExtendedHash> = scene
        .query::<&ResourcePoint>()
        .iter()
        .map(|(_, r)| r.entity)
        .filter(|v| v.is_some())
        .collect();

    let mut entity_renderers: FxHashMap<u64, EntityRenderer> = Default::default();
    for te in &to_load_entities {
        let renderer = renderer.read();
        let nh = te.hash32();
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
                    let mut cur = Cursor::new(package_manager().read_tag(e.unk0.hash())?);
                    cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x224))?;
                    let model: Tag<SEntityModel> =
                        TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;
                    cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x3c0))?;
                    let entity_material_map: Vec<Unk808072c5> =
                        TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;
                    cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x400))?;
                    let materials: Vec<TagHash> =
                        TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

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
                        e.unk0.hash()
                    )
                }
            }
        }
    }

    info!("Loading {} background entities", to_load_entitymodels.len());

    for t in to_load_entitymodels {
        let renderer = renderer.read();
        if let Ok(model) = package_manager().read_tag_struct::<SEntityModel>(t) {
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
    }

    info!(
        "Found {} entity models ({} entities)",
        entity_renderers.len(),
        to_load_entities.len()
    );

    // TODO(cohae): Maybe not the best idea?
    info!("Updating resource constant buffers");
    for (_, (transform, rp)) in scene.query_mut::<(&Transform, &mut ResourcePoint)>() {
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
        let sampler_header_ref = package_manager().get_entry(s).unwrap().reference;
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

    Ok(LoadMapData {
        scene,
        entity_renderers,
    })
}

pub struct LoadMapData {
    pub scene: Scene,
    pub entity_renderers: FxHashMap<u64, EntityRenderer>,
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
    stringmap: Arc<FxHashMap<u32, String>>,
    entity_worldid_name_map: &FxHashMap<u64, String>,

    material_map: &mut FxHashMap<TagHash, Technique>,
    to_load_entitymodels: &mut FxHashSet<TagHash>,
    unknown_root_resources: &mut FxHashMap<u32, Vec<TagHash>>,
) -> anyhow::Result<()> {
    let renderer = renderer.read();
    let dcs = renderer.dcs.clone();

    let mut ents = vec![];
    for data in &table.data_entries {
        let transform = Transform {
            translation: Vec3::new(data.translation.x, data.translation.y, data.translation.z),
            rotation: data.rotation,
            scale: Vec3::splat(data.translation.w),
            ..Default::default()
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
                                        warn!("Instance group {preheader_tag} doesn't have enough occlusion bounds, need range {}..{}, but there are only {} bounds", s.instance_start, s.instance_start + s.instance_count, preheader.instances.occlusion_bounds.bounds.len());
                                        vec![
                                            SObjectOcclusionBounds {
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

                    let terrain_resource: Unk8080714b = TigerReadable::read_ds(table_data).unwrap();
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

                    let cubemap_volume: Unk80806b7f = TigerReadable::read_ds(table_data).unwrap();
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
                        min: volume_min.truncate(),
                        max: volume_max.truncate(),
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
                            resource: MapResource::LensFlare(tag),
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
                    let tag: ExtendedHash = TigerReadable::read_ds(table_data).unwrap();
                    if !tag.is_some() || tag.hash32().is_none() {
                        // TODO: should be handled a bit more gracefully, shouldnt drop the whole node
                        // TODO: do the same for other resources ^
                        continue;
                    }

                    let header = package_manager().read_tag_struct::<Unk80809802>(tag).ok();

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
                        to_load_entitymodels.insert(unk8.unk60.entity_model);

                        if unk8.bounds != unk18.bb {
                            warn!(
                                "Bounds mismatch in Unk80806aa3: {:?} != {:?}",
                                unk8.bounds, unk18.bb
                            );
                        }

                        ents.push(scene.spawn((
                            Transform::from_mat4(Mat4::from_cols_array(&unk8.transform)),
                            ResourcePoint {
                                resource: MapResource::Unk80806aa3(
                                    unk18.bb,
                                    unk8.unk60.entity_model,
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
                                resource: MapResource::Light(
                                    bounds.bb,
                                    tag,
                                    i,
                                    light.technique_shading,
                                ),

                                entity_cbuffer: ConstantBufferCached::create_empty(dcs.clone())?,
                                ..base_rp
                            },
                            EntityWorldId(data.world_id),
                            PointLight {
                                attenuation: Vec4::ONE,
                            },
                            light.clone(),
                            bounds.bb,
                            Light,
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
                        Light,
                    )));
                }
                0x80809178 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset))
                        .unwrap();

                    let d: Unk80809178 = TigerReadable::read_ds(table_data)?;
                    let name = stringmap
                        .get(&d.area_name.0)
                        .cloned()
                        .unwrap_or_else(|| format!("[MissingString_{:08x}]", d.area_name.0));

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
                                            transform.to_mat4() * Mat4::from_translation(center),
                                        );

                                        (
                                            CustomDebugShape::from_havok_shape(&dcs, &shape).ok(),
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

                    ents.push(scene.spawn((
                        new_transform.unwrap_or(transform),
                        ResourcePoint {
                            resource: MapResource::NamedArea(d, name, havok_debugshape),
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

                    let d: Unk8080917b = TigerReadable::read_ds(table_data)?;

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

                    let resource = match d.kind {
                        0 => MapResource::InstantKillBarrier(
                            d.unk0.havok_file,
                            d.unk0.shape_index,
                            havok_debugshape,
                        ),
                        1 => MapResource::TurnbackKillBarrier(
                            d.unk0.havok_file,
                            d.unk0.shape_index,
                            havok_debugshape,
                        ),
                        _ => {
                            error!("Unknown kill barrier type {}", d.kind);
                            MapResource::InstantKillBarrier(
                                d.unk0.havok_file,
                                d.unk0.shape_index,
                                havok_debugshape,
                            )
                        }
                    };

                    ents.push(scene.spawn((
                        transform,
                        ResourcePoint {
                            resource,
                            has_havok_data: true,
                            ..base_rp
                        },
                        EntityWorldId(data.world_id),
                    )));
                }
                0x80808604 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset))
                        .unwrap();

                    let d: Unk80808604 = TigerReadable::read_ds(table_data)?;

                    let (havok_debugshape, new_transform) = if let Ok(havok_data) =
                        package_manager().read_tag(d.unk10.havok_file)
                    {
                        let mut cur = Cursor::new(&havok_data);
                        match destiny_havok::shape_collection::read_shape_collection(&mut cur) {
                            Ok(shapes) => {
                                let mut final_shape =
                                    destiny_havok::shape_collection::Shape::default();

                                for t in &d.unk10.unk8 {
                                    if t.shape_index as usize >= shapes.len() {
                                        error!(
                                            "Shape index out of bounds for Unk80808604 (table {}, {} shapes, index {})",
                                            table_hash, shapes.len(), t.shape_index
                                        );
                                        continue;
                                    }

                                    let transform = Transform {
                                        translation: t.translation.truncate(),
                                        rotation: t.rotation,
                                        ..Default::default()
                                    };

                                    let mut shape = shapes[t.shape_index as usize].clone();
                                    shape.apply_transform(transform.to_mat4());

                                    final_shape.combine(&shape);
                                }

                                // Re-center the shape
                                let center = final_shape.center();
                                final_shape.apply_transform(Mat4::from_translation(-center));

                                let new_transform = Transform {
                                    translation: center,
                                    ..Default::default()
                                };

                                (
                                    CustomDebugShape::from_havok_shape(&dcs, &final_shape).ok(),
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

                    ents.push(scene.spawn((
                        new_transform.unwrap_or(transform),
                        ResourcePoint {
                            resource: MapResource::PlayAreaBounds(
                                d.unk10.havok_file,
                                havok_debugshape,
                            ),
                            has_havok_data: true,
                            ..base_rp
                        },
                        EntityWorldId(data.world_id),
                    )));
                }
                0x80808246 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset))
                        .unwrap();

                    let d: Unk80808246 = TigerReadable::read_ds(table_data)?;

                    match package_manager().read_tag(d.unk10.havok_file) {
                        Ok(havok_data) => {
                            let mut cur = Cursor::new(&havok_data);
                            match destiny_havok::shape_collection::read_shape_collection(&mut cur) {
                                Ok(shapes) => {
                                    for t in &d.unk10.unk10 {
                                        if t.shape_index as usize >= shapes.len() {
                                            error!(
                                            "Shape index out of bounds for Unk80808246 (table {}, {} shapes, index {})",
                                            table_hash, shapes.len(), t.shape_index
                                        );
                                            continue;
                                        }

                                        let transform = Transform {
                                            translation: t.translation.truncate(),
                                            rotation: t.rotation,
                                            ..Default::default()
                                        };

                                        ents.push(
                                            scene.spawn((
                                                transform,
                                                ResourcePoint {
                                                    resource: MapResource::Unk80808246(
                                                        d.unk10.havok_file,
                                                        t.shape_index,
                                                        CustomDebugShape::from_havok_shape(
                                                            &dcs,
                                                            &shapes[t.shape_index as usize],
                                                        )
                                                        .ok(),
                                                    ),
                                                    has_havok_data: true,
                                                    entity_cbuffer:
                                                        ConstantBufferCached::create_empty(
                                                            dcs.clone(),
                                                        )?,
                                                    ..base_rp
                                                },
                                                EntityWorldId(data.world_id),
                                            )),
                                        );
                                    }

                                    // let new_transform = Transform {
                                    //     translation: center,
                                    //     ..Default::default()
                                    // };

                                    // (
                                    //     CustomDebugShape::from_havok_shape(&dcs, &final_shape).ok(),
                                    //     Some(new_transform),
                                    // )
                                }
                                Err(e) => {
                                    error!("Failed to read shapes: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to read shapes: {e}");
                        }
                    };
                }
                0x80806ac2 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset))
                        .unwrap();

                    let d: Unk80806ac2 = TigerReadable::read_ds(table_data)?;

                    match package_manager().read_tag(d.unk10.havok_file) {
                        Ok(havok_data) => {
                            let mut cur = Cursor::new(&havok_data);
                            match destiny_havok::shape_collection::read_shape_collection(&mut cur) {
                                Ok(shapes) => {
                                    if let Some(t) = d.unk10.unk10.get(d.array_index as usize) {
                                        if t.shape_index as usize >= shapes.len() {
                                            error!(
                                                "Shape index out of bounds for Unk80808246 (table {}, {} shapes, index {})",
                                                table_hash, shapes.len(), t.shape_index
                                            );

                                            continue;
                                        }

                                        let transform = Transform {
                                            translation: t.translation.truncate(),
                                            rotation: t.rotation,
                                            ..Default::default()
                                        };

                                        ents.push(
                                            scene.spawn((
                                                transform,
                                                ResourcePoint {
                                                    resource: MapResource::Unk80806ac2(
                                                        d.unk10.havok_file,
                                                        t.shape_index,
                                                        CustomDebugShape::from_havok_shape(
                                                            &dcs,
                                                            &shapes[t.shape_index as usize],
                                                        )
                                                        .ok(),
                                                    ),
                                                    has_havok_data: true,
                                                    entity_cbuffer:
                                                        ConstantBufferCached::create_empty(
                                                            dcs.clone(),
                                                        )?,
                                                    ..base_rp
                                                },
                                                EntityWorldId(data.world_id),
                                            )),
                                        );
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to read shapes: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to read shapes: {e}");
                        }
                    };
                }
                0x80809121 => {
                    table_data
                        .seek(SeekFrom::Start(data.data_resource.offset))
                        .unwrap();

                    let d: SSlipSurfaceVolume = TigerReadable::read_ds(table_data)?;

                    let (havok_debugshape, new_transform) =
                        if let Ok(havok_data) = package_manager().read_tag(d.havok_file) {
                            let mut cur = Cursor::new(&havok_data);
                            match destiny_havok::shape_collection::read_shape_collection(&mut cur) {
                                Ok(o) => {
                                    if (d.shape_index as usize) < o.len() {
                                        let mut shape = o[d.shape_index as usize].clone();

                                        let center = shape.center();
                                        shape.apply_transform(Mat4::from_translation(-center));

                                        let new_transform = Transform::from_mat4(
                                            transform.to_mat4() * Mat4::from_translation(center),
                                        );

                                        (
                                            CustomDebugShape::from_havok_shape(&dcs, &shape).ok(),
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

                    ents.push(scene.spawn((
                        new_transform.unwrap_or(transform),
                        ResourcePoint {
                            resource: MapResource::SlipSurfaceVolume(
                                d.havok_file,
                                havok_debugshape,
                            ),
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

                    let d: Unk808068d4 = TigerReadable::read_ds(table_data)?;
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
                        unknown_root_resources
                            .entry(u)
                            .or_default()
                            .push(table_hash);
                        debug!("World origin resource {} is not parsed! Resource points might be missing (table {})", TagHash(u), table_hash);
                    }

                    debug!(
                        "Skipping unknown resource type {u:x} {:?} (table file {})",
                        data.translation, table_hash
                    );
                    ents.push(scene.spawn((transform, base_rp, EntityWorldId(data.world_id))));

                    if data.data_resource.is_valid {
                        table_data
                            .seek(SeekFrom::Start(data.data_resource.offset))
                            .unwrap();

                        while let Ok(val) = table_data.read_le::<u32>() {
                            let tag = TagHash(val);
                            if tag.is_some() {
                                if let Some(entry) = package_manager().get_entry(tag) {
                                    if entry.file_type == 27 && entry.file_subtype == 0 {
                                        warn!("\t- Havok file found in unknown resource type {u:x} {:?} (table file {}, found havok file {})", data.translation, table_hash, tag);
                                    } else {
                                        // We need to go deeper
                                        if let Some(htag) = contains_havok_references(tag, 4) {
                                            warn!("\t- Havok file found in unknown resource type {u:x} {:?} (table file {table_hash}, found in subtag {tag}=>{htag})", data.translation);
                                        }
                                    }
                                }
                            }

                            // Probably hit another resource pointer
                            if (0x80800000..=0x8080ffff).contains(&val) {
                                break;
                            }
                        }
                    }
                    // warn!(
                    //     "- 0x{:08x}: {}",
                    //     data.data_resource.resource_type, data.data_resource.is_valid
                    // );
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
        if matches!(
            resource_origin,
            ResourceOriginType::Activity | ResourceOriginType::ActivityBruteforce
        ) {
            insert_tag(scene, e, EntityTag::Activity);
        }

        if resource_origin == ResourceOriginType::Ambient {
            insert_tag(scene, e, EntityTag::Ambient);
        }

        if scene
            .get::<&ResourcePoint>(e)
            .map(|r| r.has_havok_data)
            .unwrap_or_default()
        {
            insert_tag(scene, e, EntityTag::Havok);
        }

        if let Ok(transform) = scene.get::<&Transform>(e).map(|t| (*t)) {
            scene.insert_one(e, OriginalTransform(transform)).ok();
        };

        if let Ok(world_id) = scene.get::<&EntityWorldId>(e).map(|w| w.0) {
            if let Some(name) = entity_worldid_name_map.get(&world_id) {
                scene.insert_one(e, Label(name.clone())).ok();
            }
        };
    }

    Ok(())
}

fn contains_havok_references(this_tag: TagHash, max_depth: usize) -> Option<TagHash> {
    if max_depth == 0 {
        return None;
    }

    if let Ok(data) = package_manager().read_tag(this_tag) {
        let mut cursor = Cursor::new(&data);
        while let Ok(val) = cursor.read_le::<u32>() {
            let tag = TagHash(val);
            if tag.is_some() {
                if let Some(entry) = package_manager().get_entry(tag) {
                    if entry.file_type == 27 && entry.file_subtype == 0 {
                        return Some(tag);
                    } else {
                        // We need to go deeper
                        if let Some(tag) = contains_havok_references(tag, max_depth - 1) {
                            return Some(tag);
                        }
                    }
                }
            }
        }
    }

    None
}

fn is_physics_entity(entity: ExtendedHash) -> bool {
    let Ok(header) = package_manager().read_tag_struct::<Unk80809c0f>(entity) else {
        return false;
    };

    for e in &header.entity_resources {
        if e.unk0.unk10.resource_type == 0x8080916a {
            return true;
        }
    }

    false
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

    // if matches!(e.unk18.resource_type, 0x80808cf8 | 0x808098fa) {
    //     cur.seek(SeekFrom::Start(e.unk18.offset + 0x50)).ok()?;
    //     let list: TablePointer<Unk80809905> = TigerReadable::read_ds_endian(&mut cur, Endian::Little).ok()?;
    //     world_id_list = list.take_data();
    // }

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

pub fn create_map_stringmap() -> FxHashMap<TagHash, String> {
    let stringmap: FxHashMap<TagHash, String> = package_manager()
        .get_named_tags_by_class(SDestination::ID.unwrap())
        .par_iter()
        .flat_map(|(name, tag)| {
            let _span = info_span!("Read destination", destination = name).entered();
            let destination: SDestination = package_manager().read_tag_struct(*tag).unwrap();

            let destination_strings: FxHashMap<u32, String> = {
                let _span = info_span!("Read destination strings").entered();
                match StringContainer::load(destination.string_container.hash32()) {
                    Ok(sc) => sc.0,
                    Err(e) => {
                        error!("Failed to load string container: {e}");
                        FxHashMap::default()
                    }
                }
            };

            let mut strings = vec![];
            for activity_desc in &destination.activities {
                let _span = info_span!(
                    "Read activity",
                    activity = activity_desc.activity_name.0.to_string()
                )
                .entered();
                let Ok(activity) = package_manager()
                    .read_named_tag_struct::<SActivity>(activity_desc.activity_name.0.to_string())
                else {
                    continue;
                };

                for u1 in &activity.unk50 {
                    for map in &u1.map_references {
                        let map32 = match map.hash32_checked() {
                            Some(m) => m,
                            None => {
                                // error!("Couldn't translate map hash64 {map:?}");
                                continue;
                            }
                        };

                        if let Ok(bubble) =
                            package_manager().read_tag_struct::<SBubbleParentShallow>(map32)
                        {
                            let name = destination_strings
                                .get(&bubble.map_name.0)
                                .cloned()
                                .unwrap_or_else(|| {
                                    format!("[MissingString_{:08x}]", bubble.map_name.0)
                                });

                            strings.push((map32, name));
                        }
                    }
                }
            }

            strings
        })
        .collect();

    stringmap
}
