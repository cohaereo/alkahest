#[macro_use]
extern crate windows;

use std::cell::RefCell;

use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use binrw::BinReaderExt;
use destiny_pkg::PackageVersion::Destiny2PreBeyondLight;
use destiny_pkg::{PackageManager, TagHash};
use glam::{Mat4, Quat, Vec3, Vec4};
use itertools::Itertools;
use nohash_hasher::IntMap;

use strum::EnumCount;
use tracing::level_filters::LevelFilter;
use tracing::{debug, debug_span, error, info, info_span, trace, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct3D::Fxc::{
    D3DCompileFromFile, D3DCOMPILE_DEBUG, D3DCOMPILE_SKIP_OPTIMIZATION,
};
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use crate::camera::FpsCamera;
use crate::config::{WindowConfig, CONFIGURATION};
use crate::dxbc::{get_input_signature, DxbcHeader, DxbcInputType};

use crate::entity::{Unk808072c5, Unk808073a5, Unk80809c0f};
use crate::input::InputState;
use crate::map::{MapData, MapDataList, Unk80806ef4, Unk8080714f, Unk80807dae, Unk80808a54};
use crate::map_resources::{
    MapResource, Unk80806b7f, Unk80806df3, Unk80806e68, Unk8080714b, Unk80807268, Unk80809162,
};
use crate::material::{Material, Unk808071e8};
use crate::overlays::camera_settings::{CameraPositionOverlay, CurrentCubemap};
use crate::overlays::console::ConsoleOverlay;
use crate::overlays::fps_display::FpsDisplayOverlay;
use crate::overlays::gbuffer_viewer::{
    CompositorMode, CompositorOptions, GBufferInfoOverlay, COMPOSITOR_MODES,
};
use crate::overlays::gui::GuiManager;
use crate::overlays::resource_nametags::{ResourcePoint, ResourceTypeOverlay};
use crate::packages::{package_manager, PACKAGE_MANAGER};
use crate::render::scopes::ScopeRigidModel;
use crate::render::static_render::StaticModel;
use crate::render::terrain::TerrainRenderer;
use crate::render::{
    ConstantBuffer, DeviceContextSwapchain, EntityRenderer, GBuffer, InstancedRenderer, RenderData,
};
use crate::resources::Resources;
use crate::statics::{Unk808071a7, Unk8080966d};
use crate::structure::{TablePointer, Tag};
use crate::text::{decode_text, StringData, StringPart, StringSetHeader};
use crate::texture::Texture;
use crate::types::{Vector4, AABB};
use crate::vertex_layout::InputElement;
use render::scopes::ScopeView;
use crate::overlays::package_dump::PackageDumper;

mod camera;
mod config;
mod dds;
mod dxbc;
mod dxgi;
mod entity;
mod icons;
mod input;
mod map;
mod map_resources;
mod material;
mod overlays;
mod packages;
mod render;
mod resources;
mod statics;
mod structure;
mod text;
mod texture;
mod types;
mod unknown;
mod vertex_layout;

pub fn main() -> anyhow::Result<()> {
    rayon::ThreadPoolBuilder::new()
        .thread_name(|i| format!("rayon-worker-{i}"))
        .build_global()
        .unwrap();

    if let Ok(c) = std::fs::read_to_string("config.yml") {
        *CONFIGURATION.write() = serde_yaml::from_str(&c)?;
    } else {
        info!("No config found, creating a new one");
        config::persist();
    }

    let tracy_layer = if cfg!(feature = "tracy") {
        Some(tracing_tracy::TracyLayer::new())
    } else {
        None
    };

    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(tracy_layer)
            .with(overlays::console::ConsoleLogLayer)
            .with(tracing_subscriber::fmt::layer())
            .with(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            ),
    )
    .expect("Failed to set up the tracing subscriber");

    let (package, pm) = info_span!("Initializing package manager").in_scope(|| {
        let pkg_path = std::env::args().nth(1).expect("No package file was given!");
        (
            Destiny2PreBeyondLight
                .open(&pkg_path)
                .expect("Failed to open package"),
            PackageManager::new(
                PathBuf::from_str(&pkg_path).unwrap().parent().unwrap(),
                Destiny2PreBeyondLight,
                true,
            )
            .unwrap(),
        )
    });

    PACKAGE_MANAGER.with(|v| *v.borrow_mut() = Some(Rc::new(pm)));

    let mut stringmap: IntMap<u32, String> = Default::default();
    let all_global_packages = [
        0x019a, 0x01cf, 0x01fe, 0x0211, 0x0238, 0x03ab, 0x03d1, 0x03ed, 0x03f5, 0x06dc,
    ];
    {
        let _span = info_span!("Loading global strings").entered();
        for (t, _) in package_manager()
            .get_all_by_reference(0x80809a88)
            .into_iter()
            .filter(|(t, _)| all_global_packages.contains(&t.pkg_id()))
        {
            let textset_header: StringSetHeader = package_manager().read_tag_struct(t)?;

            let data = package_manager()
                .read_tag(textset_header.language_english)
                .unwrap();
            let mut cur = Cursor::new(&data);
            let text_data: StringData = cur.read_le()?;

            for (combination, hash) in text_data
                .string_combinations
                .iter()
                .zip(textset_header.string_hashes.iter())
            {
                let mut final_string = String::new();

                for ip in 0..combination.part_count {
                    cur.seek(combination.data.into())?;
                    cur.seek(SeekFrom::Current(ip * 0x20))?;
                    let part: StringPart = cur.read_le()?;
                    cur.seek(part.data.into())?;
                    let mut data = vec![0u8; part.byte_length as usize];
                    cur.read_exact(&mut data)?;
                    final_string += &decode_text(&data, part.cipher_shift);
                }

                stringmap.insert(hash.0, final_string);
            }
        }
    }

    info!("Loaded {} global strings", stringmap.len());

    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Alkahest")
        .with_inner_size(config::with(|c| {
            PhysicalSize::new(c.window.width, c.window.height)
        }))
        .with_position(config::with(|c| {
            PhysicalPosition::new(c.window.pos_x, c.window.pos_y)
        }))
        .with_maximized(config!().window.maximised)
        .build(&event_loop)?;

    // cohae: Slight concern for thread safety here. ID3D11Device is threadsafe, but ID3D11DeviceContext is *not*
    let dcs = Rc::new(DeviceContextSwapchain::create(&window)?);
    let mut gbuffer = GBuffer::create(
        (window.inner_size().width, window.inner_size().height),
        dcs.clone(),
    )?;

    let mut static_map: IntMap<u32, Arc<StaticModel>> = Default::default();
    let mut material_map: IntMap<u32, Material> = Default::default();
    let mut vshader_map: IntMap<u32, (ID3D11VertexShader, Option<ID3D11InputLayout>)> =
        Default::default();
    let mut pshader_map: IntMap<u32, ID3D11PixelShader> = Default::default();
    let mut cbuffer_map_vs: IntMap<u32, ConstantBuffer<Vector4>> = Default::default();
    let mut cbuffer_map_ps: IntMap<u32, ConstantBuffer<Vector4>> = Default::default();
    let mut texture_map: IntMap<u32, Texture> = Default::default();
    let mut sampler_map: IntMap<u32, ID3D11SamplerState> = Default::default();
    let mut terrain_headers = vec![];
    let mut maps: Vec<MapData> = vec![];

    let mut to_load_textures: HashMap<TagHash, ()> = Default::default();

    // First light reserved for camera light
    let mut point_lights = vec![Vec4::ZERO];
    for (index, _) in package.get_all_by_reference(0x80807dae) {
        let think: Unk80807dae = package_manager()
            .read_tag_struct((package.pkg_id(), index as _))
            .unwrap();

        let mut placement_groups = vec![];
        let mut resource_points = vec![];
        let mut terrains = vec![];
        for res in &think.child_map.map_resources {
            let thing2: Unk80808a54 = if res.is_hash32 != 0 {
                package_manager().read_tag_struct(res.hash32).unwrap()
            } else {
                package_manager().read_tag64_struct(res.hash64.0).unwrap()
            };

            for table in &thing2.data_tables {
                let table_data = package_manager().read_tag(table.tag()).unwrap();
                let mut cur = Cursor::new(&table_data);

                for data in &table.data_entries {
                    if data.data_resource.is_valid {
                        match data.data_resource.resource_type {
                            // D2Class_C96C8080 (placement)
                            0x808071b3 => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                                    .unwrap();
                                let preheader_tag: TagHash = cur.read_le().unwrap();
                                let preheader: Unk80806ef4 =
                                    package_manager().read_tag_struct(preheader_tag).unwrap();

                                placement_groups.push(preheader.placement_group);
                            }
                            // D2Class_7D6C8080 (terrain)
                            0x8080714b => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset))
                                    .unwrap();

                                let terrain_resource: Unk8080714b = cur.read_le().unwrap();
                                let terrain: Unk8080714f = package_manager()
                                    .read_tag_struct(terrain_resource.terrain)
                                    .unwrap();

                                for p in &terrain.mesh_parts {
                                    if p.material.is_valid() {
                                        material_map.insert(
                                            p.material.0,
                                            Material(
                                                package_manager().read_tag_struct(p.material)?,
                                                p.material,
                                            ),
                                        );
                                    }
                                }

                                terrain_headers.push((terrain_resource.terrain, terrain));
                                terrains.push(terrain_resource.terrain);
                            }
                            // Cubemap volume
                            0x80806b7f => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset))
                                    .unwrap();

                                let cubemap_volume: Unk80806b7f = cur.read_le().unwrap();
                                let extents_center = Vec4::new(
                                    cubemap_volume.cubemap_center.x,
                                    cubemap_volume.cubemap_center.y,
                                    cubemap_volume.cubemap_center.z,
                                    cubemap_volume.cubemap_center.w,
                                );
                                let extents = Vec4::new(
                                    cubemap_volume.cubemap_size.x,
                                    cubemap_volume.cubemap_size.y,
                                    cubemap_volume.cubemap_size.z,
                                    cubemap_volume.cubemap_size.w,
                                );
                                let extents_half = extents / 2.0;

                                let volume_min = extents_center - extents_half;
                                let volume_max = extents_center + extents_half;

                                to_load_textures.insert(cubemap_volume.cubemap_texture, ());
                                resource_points.push(ResourcePoint {
                                    translation: extents_center,
                                    rotation: Quat::from_xyzw(
                                        data.rotation.x,
                                        data.rotation.y,
                                        data.rotation.z,
                                        data.rotation.w,
                                    ),
                                    entity: data.entity,
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
                            // Point light
                            0x80806cbf => {
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
                                    resource_type: data.data_resource.resource_type,
                                    resource: MapResource::PointLight(tag),
                                });
                                point_lights.push(Vec4::new(
                                    data.translation.x,
                                    data.translation.y,
                                    data.translation.z,
                                    data.translation.w,
                                ));
                            }
                            // Decal collection
                            0x80806e62 => {
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
                                            resource_type: data.data_resource.resource_type,
                                            resource: MapResource::Decal {
                                                material: inst.material,
                                            },
                                        })
                                    }
                                }
                            }
                            // Unknown, every element has a mesh (material+index+vertex) and the required transforms
                            0x80806df1 => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                                    .unwrap();
                                let tag: TagHash = cur.read_le().unwrap();
                                if !tag.is_valid() {
                                    continue;
                                }

                                let header: Unk80806df3 =
                                    package_manager().read_tag_struct(tag).unwrap();

                                for p in &header.unk8 {
                                    resource_points.push(ResourcePoint {
                                        translation: Vec4::new(
                                            p.translation.x,
                                            p.translation.y,
                                            p.translation.z,
                                            p.translation.w,
                                        ),
                                        rotation: Quat::IDENTITY,
                                        entity: data.entity,
                                        resource_type: data.data_resource.resource_type,
                                        resource: MapResource::Unk80806df1,
                                    });
                                }
                            }
                            // Unknown, structure seems like that of an octree
                            0x80806f38 => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                                    .unwrap();
                                let tag: TagHash = cur.read_le().unwrap();
                                if !tag.is_valid() {
                                    continue;
                                }

                                let header: Unk80807268 =
                                    package_manager().read_tag_struct(tag).unwrap();

                                for p in &header.unk50 {
                                    resource_points.push(ResourcePoint {
                                        translation: Vec4::new(
                                            p.unk0.x, p.unk0.y, p.unk0.z, p.unk0.w,
                                        ),
                                        rotation: Quat::IDENTITY,
                                        entity: data.entity,
                                        resource_type: data.data_resource.resource_type,
                                        resource: MapResource::Unk80806f38,
                                    });
                                }
                            }
                            0x80809160 => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                                    .unwrap();
                                let tag: TagHash = cur.read_le().unwrap();
                                if !tag.is_valid() {
                                    continue;
                                }

                                let header: Unk80809162 =
                                    package_manager().read_tag_struct(tag).unwrap();

                                for p in &header.unk8 {
                                    resource_points.push(ResourcePoint {
                                        translation: Vec4::new(
                                            p.unk10.x, p.unk10.y, p.unk10.z, p.unk10.w,
                                        ),
                                        rotation: Quat::IDENTITY,
                                        entity: data.entity,
                                        resource_type: data.data_resource.resource_type,
                                        resource: MapResource::RespawnPoint,
                                    });
                                }
                            }
                            u => {
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
                                    resource_type: data.data_resource.resource_type,
                                    resource: MapResource::Unknown(
                                        data.data_resource.resource_type,
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
                            resource_type: u32::MAX,
                            resource: MapResource::Entity(data.entity),
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
            "Map {:x?} '{map_name}' - {} placement groups",
            think.map_name,
            placement_groups.len()
        );

        maps.push(MapData {
            hash: (package.pkg_id(), index as _).into(),
            name: map_name,
            placement_groups,
            resource_points,
            terrains,
        })
    }

    let to_load_entities: IntMap<TagHash, ()> = maps
        .iter()
        .flat_map(|v| v.resource_points.iter().map(|r| (r.entity, ())))
        .filter(|(v, _)| v.is_valid())
        .collect();

    let mut entity_renderers: IntMap<TagHash, EntityRenderer> = Default::default();
    for te in to_load_entities.keys().filter(|h| h.is_valid()) {
        let header: Unk80809c0f = package_manager().read_tag_struct(*te)?;
        for e in &header.unk10 {
            match e.unk0.unk18.resource_type {
                0x808072BD => {
                    let mut cur = Cursor::new(package_manager().read_tag(e.unk0.tag())?);
                    cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x1dc))?;
                    let model: Tag<Unk808073a5> = cur.read_le()?;
                    cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x300))?;
                    let entity_material_map: TablePointer<Unk808072c5> = cur.read_le()?;
                    let materials: TablePointer<Tag<Unk808071e8>> = cur.read_le()?;

                    for m in &materials {
                        material_map.insert(m.tag().0, Material(m.0.clone(), m.tag()));
                    }

                    for m in &model.meshes {
                        for p in &m.parts {
                            if p.material.is_valid() {
                                material_map.insert(
                                    p.material.0,
                                    Material(
                                        package_manager().read_tag_struct(p.material)?,
                                        p.material,
                                    ),
                                );
                            }
                        }
                    }

                    entity_renderers.insert(
                        *te,
                        EntityRenderer::load(
                            model.0,
                            entity_material_map.to_vec(),
                            materials.iter().map(|m| m.tag()).collect_vec(),
                            &dcs,
                        )?,
                    );

                    // println!(" - EntityModel {model:?}");
                }
                u => trace!(
                    "Unknown entity resource type {u:08X} (0x{:08X})",
                    e.unk0.unk10.resource_type
                ),
            }
        }
    }

    info!(
        "Found {} entity models ({} entities)",
        entity_renderers.len(),
        to_load_entities.len()
    );

    info!("{} lights", point_lights.len());

    let mut placement_groups: IntMap<u32, (Unk8080966d, Vec<InstancedRenderer>)> =
        IntMap::default();

    let mut to_load: HashMap<TagHash, ()> = Default::default();
    let mut to_load_samplers: HashMap<TagHash, ()> = Default::default();
    for m in &maps {
        for placements in m.placement_groups.iter() {
            for v in &placements.statics {
                to_load.insert(*v, ());
            }
            placement_groups.insert(placements.tag().0, (placements.0.clone(), vec![]));
        }
    }

    if placement_groups.is_empty() {
        panic!("No map placements found in package");
    }

    let mut terrain_renderers: IntMap<u32, TerrainRenderer> = Default::default();
    info_span!("Loading terrain").in_scope(|| {
        for (t, header) in terrain_headers.into_iter() {
            for t in &header.mesh_groups {
                to_load_textures.insert(t.dyemap, ());
            }

            match TerrainRenderer::load(header, &dcs.device) {
                Ok(renderer) => {
                    terrain_renderers.insert(t.0, renderer);
                }
                Err(e) => {
                    error!("Failed to load terrain: {e}");
                }
            }
        }
    });

    let to_load_statics: Vec<TagHash> = to_load.keys().cloned().collect();

    info_span!("Loading statics").in_scope(|| {
        for almostloadable in &to_load_statics {
            let mheader: Unk808071a7 = package_manager().read_tag_struct(*almostloadable).unwrap();
            for m in &mheader.materials {
                if m.is_valid() {
                    material_map.insert(
                        m.0,
                        Material(package_manager().read_tag_struct(*m).unwrap(), *m),
                    );
                }
            }

            match StaticModel::load(mheader, &dcs.device) {
                Ok(model) => {
                    static_map.insert(almostloadable.0, Arc::new(model));
                }
                Err(e) => {
                    error!(model = ?almostloadable, "Failed to load model: {e}");
                }
            }
        }
    });

    info!("Loaded {} statics", static_map.len());

    info_span!("Constructing instance renderers").in_scope(|| {
        let mut total_instance_data = 0;
        for (placements, renderers) in placement_groups.values_mut() {
            for instance in &placements.instances {
                if let Some(model_hash) =
                    placements.statics.iter().nth(instance.static_index as _)
                {
                    let _span =
                        debug_span!("Draw static instance", count = instance.instance_count, model = ?model_hash)
                            .entered();

                    if let Some(model) = static_map.get(&model_hash.0) {
                        let transforms = &placements.transforms[instance.instance_offset
                            as usize
                            ..(instance.instance_offset + instance.instance_count) as usize];

                        renderers.push(InstancedRenderer::load(model.clone(), transforms, dcs.clone()).unwrap());
                    }

                    total_instance_data += instance.instance_count as usize * 16 * 4;
                }
            }
        }
        debug!("Total instance data: {}kb", total_instance_data / 1024);
    });

    let mut vshader_fullscreen = None;
    let mut pshader_fullscreen = None;
    let mut errors = None;

    let flags = if cfg!(debug_assertions) {
        D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION
    } else {
        0
    };
    unsafe {
        (
            D3DCompileFromFile(
                w!("fullscreen.hlsl"),
                None,
                None,
                s!("VShader"),
                s!("vs_5_0"),
                flags,
                0,
                &mut vshader_fullscreen,
                Some(&mut errors),
            )
            .context("Failed to compile vertex shader")?,
            D3DCompileFromFile(
                w!("fullscreen.hlsl"),
                None,
                None,
                s!("PShader"),
                s!("ps_5_0"),
                flags,
                0,
                &mut pshader_fullscreen,
                Some(&mut errors),
            )
            .context("Failed to compile pixel shader")?,
        )
    };

    if let Some(errors) = errors {
        let estr = unsafe {
            let eptr = errors.GetBufferPointer();
            std::slice::from_raw_parts(eptr.cast(), errors.GetBufferSize())
        };
        let errors = String::from_utf8_lossy(estr);
        warn!("{}", errors);
    }

    let vshader_fullscreen = vshader_fullscreen.unwrap();
    let pshader_fullscreen = pshader_fullscreen.unwrap();

    info_span!("Loading shaders").in_scope(|| {
        for (t, m) in material_map.iter() {
            for sampler in m.vs_samplers.iter().chain(m.ps_samplers.iter()) {
                to_load_samplers.insert(sampler.sampler, ());
            }

            if let Ok(v) = package_manager().get_entry(m.vertex_shader) {
                let _span = debug_span!("load vshader", shader = ?m.vertex_shader).entered();

                vshader_map.entry(m.vertex_shader.0).or_insert_with(|| {
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
                    let layout = vertex_layout::build_input_layout(&layout_converted);
                    unsafe {
                        let v = dcs
                            .device
                            .CreateVertexShader(&vs_data, None)
                            .context("Failed to load vertex shader")
                            .unwrap();

                        let name = format!("VS {:?} (mat 0x{:x})\0", m.vertex_shader, t);
                        v.SetPrivateData(
                            &WKPDID_D3DDebugObjectName,
                            name.len() as u32 - 1,
                            Some(name.as_ptr() as _),
                        )
                        .expect("Failed to set VS name");

                        let input_layout = dcs.device.CreateInputLayout(&layout, &vs_data).ok();
                        if input_layout.is_none() {
                            let layout_string = layout_converted
                                .iter()
                                .enumerate()
                                .map(|(i, e)| {
                                    format!(
                                        "\t{}{} v{i} : {}{}",
                                        e.component_type,
                                        e.component_count,
                                        e.semantic_type.to_pcstr().display(),
                                        e.semantic_index
                                    )
                                })
                                .join("\n");

                            error!(
                                "Failed to load vertex layout for VS {:?}, layout:\n{}\n",
                                m.vertex_shader, layout_string
                            );
                        }

                        (v, input_layout)
                    }
                });
            }

            // return Ok(());

            if let Ok(v) = package_manager().get_entry(m.pixel_shader) {
                let _span = debug_span!("load pshader", shader = ?m.pixel_shader).entered();

                pshader_map.entry(m.pixel_shader.0).or_insert_with(|| {
                    let ps_data = package_manager().read_tag(v.reference).unwrap();
                    unsafe {
                        let v = dcs
                            .device
                            .CreatePixelShader(&ps_data, None)
                            .context("Failed to load pixel shader")
                            .unwrap();

                        let name = format!("PS {:?} (mat 0x{:x})\0", m.pixel_shader, t);
                        v.SetPrivateData(
                            &WKPDID_D3DDebugObjectName,
                            name.len() as u32 - 1,
                            Some(name.as_ptr() as _),
                        )
                        .expect("Failed to set VS name");

                        v
                    }
                });
            }

            if m.unk98.len() > 1
                && m.unk98
                    .iter()
                    .any(|v| v.x != 0.0 || v.y != 0.0 || v.z != 0.0 || v.w != 0.0)
            {
                trace!("Loading float4 cbuffer with {} elements", m.unk318.len());
                let buf = ConstantBuffer::create_array_init(dcs.clone(), &m.unk98).unwrap();

                cbuffer_map_vs.insert(*t, buf);
            }

            if m.unk34c.is_valid() {
                let buffer_header_ref = package_manager().get_entry(m.unk34c).unwrap().reference;

                let buffer = package_manager().read_tag(buffer_header_ref).unwrap();
                trace!(
                    "Read {} bytes cbuffer from {buffer_header_ref:?}",
                    buffer.len()
                );
                let buf =
                    ConstantBuffer::create_array_init(dcs.clone(), bytemuck::cast_slice(&buffer))
                        .unwrap();

                cbuffer_map_ps.insert(*t, buf);
            } else if !m.unk318.is_empty()
                && m.unk318
                    .iter()
                    .any(|v| v.x != 0.0 || v.y != 0.0 || v.z != 0.0 || v.w != 0.0)
            {
                trace!("Loading float4 cbuffer with {} elements", m.unk318.len());
                let buf = ConstantBuffer::create_array_init(dcs.clone(), &m.unk318).unwrap();

                cbuffer_map_ps.insert(*t, buf);
            }
        }
    });

    info!(
        "Loaded {} vertex shaders, {} pixel shaders",
        vshader_map.len(),
        pshader_map.len()
    );

    let (vshader_fullscreen, pshader_fullscreen) = unsafe {
        let vs_blob = std::slice::from_raw_parts(
            vshader_fullscreen.GetBufferPointer() as *const u8,
            vshader_fullscreen.GetBufferSize(),
        );
        let v2 = dcs.device.CreateVertexShader(vs_blob, None)?;
        let ps_blob = std::slice::from_raw_parts(
            pshader_fullscreen.GetBufferPointer() as *const u8,
            pshader_fullscreen.GetBufferSize(),
        );
        let v3 = dcs.device.CreatePixelShader(ps_blob, None)?;
        (v2, v3)
    };

    for m in material_map.values() {
        for t in m.ps_textures.iter().chain(m.vs_textures.iter()) {
            to_load_textures.insert(t.texture, ());
        }
    }

    let to_load_textures: Vec<TagHash> = to_load_textures.keys().cloned().collect();
    info_span!("Loading textures").in_scope(|| {
        for tex_hash in to_load_textures.into_iter() {
            if !tex_hash.is_valid() || texture_map.contains_key(&tex_hash.0) {
                continue;
            }
            let _span = debug_span!("load texture", texture = ?tex_hash).entered();

            texture_map.insert(tex_hash.0, Texture::load(&dcs, tex_hash).unwrap());
        }
    });

    info!("Loaded {} textures", texture_map.len());

    let to_load_samplers: Vec<TagHash> = to_load_samplers.keys().cloned().collect();
    for s in to_load_samplers {
        let sampler_header_ref = package_manager().get_entry(s).unwrap().reference;
        let sampler_data = package_manager().read_tag(sampler_header_ref).unwrap();

        let sampler = unsafe {
            dcs.device
                .CreateSamplerState(sampler_data.as_ptr() as _)
                .expect("Failed to create sampler state")
        };

        sampler_map.insert(s.0, sampler);
    }

    info!("Loaded {} samplers", sampler_map.len());

    let le_sampler = unsafe {
        dcs.device.CreateSamplerState(&D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_WRAP,
            AddressV: D3D11_TEXTURE_ADDRESS_WRAP,
            AddressW: D3D11_TEXTURE_ADDRESS_WRAP,
            MipLODBias: 0.,
            MaxAnisotropy: 1,
            ComparisonFunc: D3D11_COMPARISON_ALWAYS,
            BorderColor: Default::default(),
            MinLOD: 0.,
            MaxLOD: f32::MAX,
        })?
    };

    let le_terrain_cb11 = ConstantBuffer::<Mat4>::create(dcs.clone(), None)?;
    let le_entity_cb11 = ConstantBuffer::<ScopeRigidModel>::create(dcs.clone(), None)?;

    let le_vertex_cb12 = ConstantBuffer::<ScopeView>::create(dcs.clone(), None)?;
    let le_entity_cb13 = ConstantBuffer::<Vec4>::create(dcs.clone(), None)?;

    let cb_composite_lights =
        ConstantBuffer::<Vec4>::create_array_init(dcs.clone(), &point_lights)?;

    let cb_composite_options = ConstantBuffer::<CompositorOptions>::create(dcs.clone(), None)?;

    let rasterizer_state = unsafe {
        dcs.device
            .CreateRasterizerState(&D3D11_RASTERIZER_DESC {
                FillMode: D3D11_FILL_SOLID,
                CullMode: D3D11_CULL_BACK,
                FrontCounterClockwise: true.into(),
                DepthBias: 0,
                DepthBiasClamp: 0.0,
                SlopeScaledDepthBias: 0.0,
                DepthClipEnable: true.into(),
                ScissorEnable: Default::default(),
                MultisampleEnable: Default::default(),
                AntialiasedLineEnable: Default::default(),
            })
            .context("Failed to create Rasterizer State")?
    };

    let mut resources: Resources = Resources::default();
    resources.insert(FpsCamera::default());
    resources.insert(InputState::default());
    resources.insert(MapDataList {
        current_map: 0,
        maps,
    });
    // TODO(cohae): This is fucking terrible, just move it to the debug GUI when we can
    resources.insert(CurrentCubemap(None));

    let matcap = unsafe {
        const MATCAP_DATA: &[u8] = include_bytes!("matte.data");
        dcs.device
            .CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: 128 as _,
                    Height: 128 as _,
                    MipLevels: 1,
                    ArraySize: 1 as _,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_SHADER_RESOURCE,
                    CPUAccessFlags: Default::default(),
                    MiscFlags: Default::default(),
                },
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: MATCAP_DATA.as_ptr() as _,
                    SysMemPitch: 128 * 4,
                    ..Default::default()
                } as _),
            )
            .context("Failed to create texture")?
    };
    let matcap_view = unsafe {
        dcs.device.CreateShaderResourceView(
            &matcap,
            Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_SRV {
                        MostDetailedMip: 0,
                        MipLevels: 1,
                    },
                },
            }),
        )?
    };

    let blend_state = unsafe {
        dcs.device.CreateBlendState(&D3D11_BLEND_DESC {
            RenderTarget: [D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: false.into(),
                SrcBlend: D3D11_BLEND_ONE,
                DestBlend: D3D11_BLEND_ZERO,
                BlendOp: D3D11_BLEND_OP_ADD,
                SrcBlendAlpha: D3D11_BLEND_ONE,
                DestBlendAlpha: D3D11_BLEND_ZERO,
                BlendOpAlpha: D3D11_BLEND_OP_ADD,
                RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
            }; 8],
            ..Default::default()
        })?
    };

    let gui_fps = Rc::new(RefCell::new(FpsDisplayOverlay::default()));
    let gui_gbuffer = Rc::new(RefCell::new(GBufferInfoOverlay {
        composition_mode: CompositorMode::Combined as usize,
        renderlayer_statics: true,
        renderlayer_terrain: true,
        renderlayer_entities: true,
    }));
    let gui_debug = Rc::new(RefCell::new(CameraPositionOverlay {
        show_map_resources: false,
        show_map_resource_label: true,
        map_resource_filter: {
            let mut f = [false; MapResource::COUNT];
            f[0] = true;
            f
        },
        map_resource_distance: 2000.0,
        render_scale: 100.0,
        render_scale_changed: false,
        render_lights: false,
    }));

    let gui_resources = Rc::new(RefCell::new(ResourceTypeOverlay {
        debug_overlay: gui_debug.clone(),
    }));

    let gui_dump = Rc::new(RefCell::new(PackageDumper::new()));

    let mut gui = GuiManager::create(&window, &dcs.device);
    let gui_console = Rc::new(RefCell::new(ConsoleOverlay::default()));
    gui.add_overlay(gui_fps);
    gui.add_overlay(gui_debug.clone());
    gui.add_overlay(gui_gbuffer.clone());
    gui.add_overlay(gui_resources.clone());
    gui.add_overlay(gui_console);
    gui.add_overlay(gui_dump.clone());

    // TODO(cohae): resources should be added to renderdata directly
    let render_data = RenderData {
        materials: material_map,
        vshaders: vshader_map,
        pshaders: pshader_map,
        cbuffers_vs: cbuffer_map_vs,
        cbuffers_ps: cbuffer_map_ps,
        textures: texture_map,
        samplers: sampler_map,
    };

    let start_time = Instant::now();
    let mut last_frame = Instant::now();
    let mut last_cursor_pos: Option<PhysicalPosition<f64>> = None;

    event_loop.run(move |event, _, control_flow| {
        gui.handle_event(&event, &window);
        resources
            .get_mut::<InputState>()
            .unwrap()
            .handle_event(&event);

        match &event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(new_dims) => unsafe {
                    *dcs.swapchain_target.write() = None;
                    dcs.swap_chain
                        .ResizeBuffers(
                            1,
                            new_dims.width,
                            new_dims.height,
                            DXGI_FORMAT_B8G8R8A8_UNORM,
                            0,
                        )
                        .expect("Failed to resize swapchain");

                    let bb: ID3D11Texture2D = dcs.swap_chain.GetBuffer(0).unwrap();

                    let new_rtv = dcs.device.CreateRenderTargetView(&bb, None).unwrap();

                    dcs.context
                        .OMSetRenderTargets(Some(&[Some(new_rtv.clone())]), None);

                    *dcs.swapchain_target.write() = Some(new_rtv);

                    let render_scale = gui_debug.borrow().render_scale / 100.0;
                    gbuffer
                        .resize((
                            (new_dims.width as f32 * render_scale) as u32,
                            (new_dims.height as f32 * render_scale) as u32,
                        ))
                        .expect("Failed to resize GBuffers");
                },
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if let Some(ref mut p) = last_cursor_pos {
                        let delta = (position.x - p.x, position.y - p.y);
                        let input = resources.get::<InputState>().unwrap();
                        if input.mouse_left() && !gui.imgui.io().want_capture_mouse {
                            let mut camera = resources.get_mut::<FpsCamera>().unwrap();
                            camera.update_mouse((delta.0 as f32, delta.1 as f32).into());
                        }

                        last_cursor_pos = Some(*position);
                    } else {
                        last_cursor_pos = Some(*position);
                    }
                }
                // TODO(cohae): Should this even be in here at this point?
                WindowEvent::KeyboardInput { .. } => {
                    let input = resources.get::<InputState>().unwrap();
                    if input.ctrl() && input.is_key_down(VirtualKeyCode::Q) {
                        *control_flow = ControlFlow::Exit
                    }
                }

                _ => (),
            },
            Event::RedrawRequested(..) => {
                let render_scale = gui_debug.borrow().render_scale / 100.0;
                if gui_debug.borrow().render_scale_changed {
                    let dims = window.inner_size();
                    gbuffer
                        .resize((
                            (dims.width as f32 * render_scale) as u32,
                            (dims.height as f32 * render_scale) as u32,
                        ))
                        .expect("Failed to resize GBuffers");
                    // Just to be safe
                    gui_debug.borrow_mut().render_scale_changed = false;
                }

                let mut camera = resources.get_mut::<FpsCamera>().unwrap();
                if !gui.imgui.io().want_capture_keyboard {
                    let input_state = resources.get::<InputState>().unwrap();
                    camera.update(&input_state, last_frame.elapsed().as_secs_f32());
                }
                last_frame = Instant::now();

                let window_dims = window.inner_size();

                unsafe {
                    dcs.context.ClearRenderTargetView(
                        &gbuffer.rt0.render_target,
                        [0.0, 0.0, 0.0, 1.0].as_ptr() as _,
                    );
                    dcs.context.ClearRenderTargetView(
                        &gbuffer.rt1.render_target,
                        [0.0, 0.0, 0.0, 0.0].as_ptr() as _,
                    );
                    dcs.context.ClearRenderTargetView(
                        &gbuffer.rt2.render_target,
                        [0.0, 0.0, 0.0, 0.0].as_ptr() as _,
                    );
                    dcs.context.ClearDepthStencilView(
                        &gbuffer.depth.view,
                        D3D11_CLEAR_DEPTH.0 as _,
                        0.0,
                        0,
                    );

                    dcs.context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                        TopLeftX: 0.0,
                        TopLeftY: 0.0,
                        Width: window_dims.width as f32 * render_scale,
                        Height: window_dims.height as f32 * render_scale,
                        MinDepth: 0.0,
                        MaxDepth: 1.0,
                    }]));

                    dcs.context.RSSetState(&rasterizer_state);
                    dcs.context.OMSetBlendState(
                        &blend_state,
                        Some(&[1f32, 1., 1., 1.] as _),
                        0xffffffff,
                    );
                    dcs.context.OMSetRenderTargets(
                        Some(&[
                            Some(gbuffer.rt0.render_target.clone()),
                            Some(gbuffer.rt1.render_target.clone()),
                            Some(gbuffer.rt2.render_target.clone()),
                        ]),
                        &gbuffer.depth.view,
                    );
                    dcs.context.OMSetDepthStencilState(&gbuffer.depth.state, 0);

                    let projection = Mat4::perspective_infinite_reverse_rh(
                        90f32.to_radians(),
                        window_dims.width as f32 / window_dims.height as f32,
                        0.0001,
                    );

                    let view = camera.calculate_matrix();

                    let proj_view = projection * view;
                    let mut view2 = Mat4::IDENTITY;
                    view2.w_axis = camera.position.extend(1.0);

                    let scope_view = ScopeView {
                        world_to_projective: proj_view,
                        camera_to_world: view2,
                        // Account for missing depth value in output
                        view_miscellaneous: Vec4::new(0.0, 0.0, 0.0001, 0.0),
                        ..Default::default()
                    };
                    le_vertex_cb12.write(&scope_view).unwrap();

                    dcs.context
                        .VSSetConstantBuffers(12, Some(&[Some(le_vertex_cb12.buffer().clone())]));

                    dcs.context
                        .PSSetConstantBuffers(12, Some(&[Some(le_vertex_cb12.buffer().clone())]));

                    let maps = resources.get::<MapDataList>().unwrap();
                    let map = &maps.maps[maps.current_map % maps.maps.len()];

                    {
                        let gb = gui_gbuffer.borrow();

                        if gb.renderlayer_statics {
                            for ptag in &map.placement_groups {
                                let (_placements, instance_renderers) =
                                    &placement_groups[&ptag.tag().0];
                                for instance in instance_renderers.iter() {
                                    instance.draw(&dcs, &render_data).unwrap();
                                }
                            }
                        }

                        if gb.renderlayer_terrain {
                            for th in &map.terrains {
                                if let Some(t) = terrain_renderers.get(&th.0) {
                                    t.draw(&dcs, &render_data, le_terrain_cb11.buffer())
                                        .unwrap();
                                }
                            }
                        }

                        if gb.renderlayer_entities {
                            le_entity_cb13
                                .write(&Vec4::splat(start_time.elapsed().as_secs_f32()))
                                .unwrap();
                            dcs.context.VSSetConstantBuffers(
                                13,
                                Some(&[Some(le_entity_cb13.buffer().clone())]),
                            );

                            dcs.context.PSSetShaderResources(
                                10,
                                Some(&[Some(gbuffer.depth.texture_view.clone())]),
                            );
                            for rp in &map.resource_points {
                                if let Some(ent) = entity_renderers.get(&rp.entity) {
                                    let mm = Mat4::from_scale_rotation_translation(
                                        Vec3::splat(rp.translation.w),
                                        rp.rotation.inverse(),
                                        Vec3::ZERO,
                                    );
                                    let model_matrix = Mat4::from_cols(
                                        mm.x_axis.truncate().extend(rp.translation.x),
                                        mm.y_axis.truncate().extend(rp.translation.y),
                                        mm.z_axis.truncate().extend(rp.translation.z),
                                        mm.w_axis,
                                    );

                                    le_entity_cb11
                                        .write(&ScopeRigidModel {
                                            mesh_to_world: model_matrix.transpose(),
                                            position_scale: ent.mesh_scale(),
                                            position_offset: ent.mesh_offset(),
                                            texcoord0_scale_offset: ent.texcoord_transform(),
                                            dynamic_sh_ao_values: Vec4::ZERO,
                                        })
                                        .unwrap();

                                    dcs.context.VSSetConstantBuffers(
                                        11,
                                        Some(&[Some(le_entity_cb11.buffer().clone())]),
                                    );

                                    ent.draw(&dcs, &render_data);
                                }
                            }
                        }
                    }

                    dcs.context.OMSetRenderTargets(
                        Some(&[Some(dcs.swapchain_target.read().as_ref().unwrap().clone())]),
                        None,
                    );
                    dcs.context.PSSetShaderResources(
                        0,
                        Some(&[
                            Some(gbuffer.rt0.view.clone()),
                            Some(gbuffer.rt1.view.clone()),
                            Some(gbuffer.rt2.view.clone()),
                            Some(gbuffer.depth.texture_view.clone()),
                            Some(matcap_view.clone()),
                        ]),
                    );

                    let compositor_options = CompositorOptions {
                        proj_view_matrix_inv: proj_view.inverse(),
                        proj_matrix: projection,
                        view_matrix: view,
                        camera_pos: camera.position.extend(1.0),
                        camera_dir: camera.front.extend(1.0),
                        mode: COMPOSITOR_MODES[gui_gbuffer.borrow().composition_mode] as u32,
                        light_count: if gui_debug.borrow().render_lights {
                            point_lights.len() as u32
                        } else {
                            0
                        },
                    };
                    cb_composite_options.write(&compositor_options).unwrap();

                    dcs.context.VSSetConstantBuffers(
                        0,
                        Some(&[Some(cb_composite_options.buffer().clone())]),
                    );

                    dcs.context.PSSetConstantBuffers(
                        0,
                        Some(&[
                            Some(cb_composite_options.buffer().clone()),
                            Some(cb_composite_lights.buffer().clone()),
                        ]),
                    );

                    dcs.context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                        TopLeftX: 0.0,
                        TopLeftY: 0.0,
                        Width: window_dims.width as f32,
                        Height: window_dims.height as f32,
                        MinDepth: 0.0,
                        MaxDepth: 1.0,
                    }]));

                    let cubemap_texture = if let Some(MapResource::CubemapVolume(c, _)) = map
                        .resource_points
                        .iter()
                        .find(|r| {
                            if let MapResource::CubemapVolume(_, aabb) = &r.resource {
                                aabb.contains_point(camera.position)
                            } else {
                                false
                            }
                        })
                        .map(|r| &r.resource)
                    {
                        if let Some(mut cr) = resources.get_mut::<CurrentCubemap>() {
                            cr.0 = Some(c.cubemap_name.to_string());
                        }
                        render_data
                            .textures
                            .get(&c.cubemap_texture.0)
                            .map(|t| t.view.clone())
                    } else {
                        if let Some(mut cr) = resources.get_mut::<CurrentCubemap>() {
                            cr.0 = None;
                        }
                        None
                    };

                    dcs.context
                        .PSSetShaderResources(5, Some(&[cubemap_texture]));

                    dcs.context
                        .PSSetSamplers(0, Some(&[Some(le_sampler.clone())]));

                    dcs.context.VSSetShader(&vshader_fullscreen, None);
                    dcs.context.PSSetShader(&pshader_fullscreen, None);
                    dcs.context
                        .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
                    dcs.context.Draw(4, 0);

                    drop(camera);
                    drop(maps);
                    gui.draw_frame(&window, last_frame.elapsed(), &mut resources);

                    dcs.context.OMSetDepthStencilState(None, 0);

                    dcs.swap_chain.Present(1, 0).unwrap();

                    if let Some(c) = tracy_client::Client::running() {
                        c.frame_mark()
                    }
                };
            }
            Event::MainEventsCleared => {
                let io = gui.imgui.io_mut();
                gui.platform
                    .prepare_frame(io, &window)
                    .expect("Failed to start frame");
                window.request_redraw();
            }
            Event::LoopDestroyed => {
                config::with_mut(|c| {
                    let size = window.inner_size();
                    let pos = window
                        .outer_position()
                        .unwrap_or(PhysicalPosition::default());
                    c.window = WindowConfig {
                        width: size.width,
                        height: size.height,
                        pos_x: pos.x,
                        pos_y: pos.y,
                        maximised: window.is_maximized(),
                    };
                });
                config::persist();
            }
            _ => (),
        }
    });
}
