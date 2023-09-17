#[macro_use]
extern crate windows;

#[macro_use]
extern crate tracing;

use std::cell::RefCell;

use std::collections::{HashMap, HashSet};

use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context;
use binrw::BinReaderExt;
use destiny_pkg::PackageVersion::{self};
use destiny_pkg::{PackageManager, TagHash};
use glam::{Quat, Vec4};
use itertools::Itertools;
use nohash_hasher::IntMap;

use strum::EnumCount;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

use windows::Win32::Foundation::DXGI_STATUS_OCCLUDED;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::{Common::*, DXGI_PRESENT_TEST, DXGI_SWAP_EFFECT_SEQUENTIAL};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use crate::camera::FpsCamera;
use crate::config::{WindowConfig, CONFIGURATION};
use crate::dxbc::{get_input_signature, get_output_signature, DxbcHeader, DxbcInputType};

use crate::input::InputState;
use crate::map::{
    ExtendedHash, MapData, MapDataList, Unk80806ef4, Unk8080714f, Unk80807dae, Unk80808a54,
};
use crate::map_resources::{MapResource, Unk80806e68, Unk8080714b};
use crate::material::Material;
use crate::overlays::camera_settings::{CameraPositionOverlay, CurrentCubemap};
use crate::overlays::console::ConsoleOverlay;
use crate::overlays::fps_display::FpsDisplayOverlay;
use crate::overlays::gui::GuiManager;
use crate::overlays::load_indicator::LoadIndicatorOverlay;
use crate::overlays::render_settings::{CompositorMode, RenderSettingsOverlay};
use crate::overlays::resource_nametags::{ResourcePoint, ResourceTypeOverlay};
use crate::overlays::tag_dump::TagDumper;
use crate::packages::{package_manager, PACKAGE_MANAGER};
use crate::render::debug::DebugShapes;
use crate::render::error::ErrorRenderer;
use crate::render::renderer::{Renderer, ScopeOverrides};

use crate::render::static_render::StaticModel;
use crate::render::terrain::TerrainRenderer;
use crate::render::{ConstantBuffer, DeviceContextSwapchain, InstancedRenderer};
use crate::resources::Resources;
use crate::statics::{Unk808071a7, Unk8080966d};

use crate::text::{decode_text, StringData, StringPart, StringSetHeader};

use render::vertex_layout::InputElement;

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
mod panic_handler;
mod render;
mod resources;
mod statics;
mod structure;
mod text;
mod texture;
mod types;
mod unknown;
mod util;

pub fn main() -> anyhow::Result<()> {
    panic_handler::install_hook();

    #[cfg(debug_assertions)]
    std::env::set_var("RUST_BACKTRACE", "1");

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
            PackageVersion::Destiny2Lightfall
                .open(&pkg_path)
                .expect("Failed to open package"),
            PackageManager::new(
                PathBuf::from_str(&pkg_path).unwrap().parent().unwrap(),
                PackageVersion::Destiny2Lightfall,
                true,
            )
            .unwrap(),
        )
    });

    *PACKAGE_MANAGER.write() = Some(Arc::new(pm));

    let mut stringmap: IntMap<u32, String> = Default::default();
    let all_global_packages = [
        0x012d, 0x0195, 0x0196, 0x0197, 0x0198, 0x0199, 0x019a, 0x019b, 0x019c, 0x019d, 0x019e,
        0x03dd,
    ];
    {
        let _span = info_span!("Loading global strings").entered();
        for (t, _) in package_manager()
            .get_all_by_reference(u32::from_be(0xEF998080))
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

    // for (t, _) in package_manager().get_all_by_reference(0x80806cb1) {
    //     let unk: Unk80806cb1 = package_manager().read_tag_struct(t)?;

    //     for m in &unk.unk20 {
    //         if !m.unkc.is_valid() {
    //             warn!("Pipeline '{}' doesn't have a material", m.name.to_string());
    //             continue;
    //         }
    //         let material: Unk808071e8 = package_manager().read_tag_struct(m.unkc)?;

    //         println!(
    //             "Extracting '{}' (vs={}, ps={}, {} textures)",
    //             *m.name,
    //             material.vertex_shader.is_valid(),
    //             material.pixel_shader.is_valid(),
    //             // material.compute_shader.is_valid(),
    //             material.vs_textures.len() + material.ps_textures.len() // + material.cs_textures.len()
    //         );

    //         let pipeline_dir = PathBuf::from_str("./pipelines/")
    //             .unwrap()
    //             .join(m.name.to_string());
    //         std::fs::create_dir_all(&pipeline_dir)?;

    //         if material.vertex_shader.is_valid() {
    //             let header_entry = package_manager().get_entry(material.vertex_shader)?;
    //             let data = package_manager().read_tag(TagHash(header_entry.reference))?;
    //             File::create(&pipeline_dir.join("vertex.cso"))?.write_all(&data)?;
    //         }

    //         if material.pixel_shader.is_valid() {
    //             let header_entry = package_manager().get_entry(material.pixel_shader)?;
    //             let data = package_manager().read_tag(TagHash(header_entry.reference))?;
    //             File::create(&pipeline_dir.join("pixel.cso"))?.write_all(&data)?;
    //         }

    //         // if material.compute_shader.is_valid() {
    //         //     let header_entry = package_manager.get_entry_by_tag(material.compute_shader)?;
    //         //     let data = package_manager.read_tag(TagHash(header_entry.reference))?;
    //         //     File::create(&pipeline_dir.join("compute.cso"))?.write_all(&data)?;
    //         // }

    //         let mut out = File::create(pipeline_dir.join("material.txt"))?;
    //         write!(&mut out, "{material:#x?}")?;

    //         let mut out = File::create(pipeline_dir.join("material.bin"))?;
    //         let data = package_manager().read_tag(m.unkc)?;
    //         out.write_all(&data)?;
    //     }

    //     // println!("{unk:#?}");
    // }

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

    let dcs = Arc::new(DeviceContextSwapchain::create(&window)?);

    // TODO(cohae): resources should be added to renderdata directly
    let mut renderer = Renderer::create(&window, dcs.clone())?;

    let mut static_map: IntMap<TagHash, Arc<StaticModel>> = Default::default();
    let mut material_map: IntMap<TagHash, Material> = Default::default();
    let mut vshader_map: IntMap<TagHash, (ID3D11VertexShader, Vec<InputElement>, Vec<u8>)> =
        Default::default();
    let mut pshader_map: IntMap<TagHash, (ID3D11PixelShader, Vec<InputElement>)> =
        Default::default();
    let mut sampler_map: IntMap<u64, ID3D11SamplerState> = Default::default();
    let mut terrain_headers = vec![];
    let mut maps: Vec<MapData> = vec![];

    // for (tag, entry) in package_manager().get_all_by_reference(u32::from_be(0x1E898080)) {
    //     println!("{} - {tag}", package_manager().package_paths[&tag.pkg_id()]);
    // }

    // First light reserved for camera light
    let point_lights = vec![Vec4::ZERO, Vec4::ZERO];
    for (index, _) in package.get_all_by_reference(u32::from_be(0x1E898080)) {
        let hash = TagHash::new(package.pkg_id(), index as _);
        let _span = debug_span!("Load map", %hash).entered();
        let think: Unk80807dae = package_manager().read_tag_struct(hash).unwrap();

        // if stringmap.get(&think.map_name.0) != Some(&"Quagmire".to_string()) {
        //     continue;
        // }

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
                            // 0x808071ad => {
                            //     cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                            //         .unwrap();
                            //     let header_tag: TagHash = cur.read_le().unwrap();
                            //     let header: Unk80807164 =
                            //         package_manager().read_tag_struct(header_tag).unwrap();

                            //     resource_points.push(ResourcePoint {
                            //         translation: Vec4::new(
                            //             (header.unk70.x + header.unk80.x) / 2.,
                            //             (header.unk70.y + header.unk80.y) / 2.,
                            //             (header.unk70.z + header.unk80.z) / 2.,
                            //             (header.unk70.w + header.unk80.w) / 2.,
                            //         ),
                            //         rotation: Quat::IDENTITY,
                            //         entity: data.entity,
                            //         resource_type: data.data_resource.resource_type,
                            //         resource: MapResource::Unk808071ad(AABB {
                            //             min: Vec3A::new(
                            //                 header.unk70.x,
                            //                 header.unk70.y,
                            //                 header.unk70.z,
                            //             ),
                            //             max: Vec3A::new(
                            //                 header.unk80.x,
                            //                 header.unk80.y,
                            //                 header.unk80.z,
                            //             ),
                            //         }),
                            //     });
                            // }
                            // // D2Class_7D6C8080 (terrain)
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
                            // // Cubemap volume
                            // 0x80806b7f => {
                            //     cur.seek(SeekFrom::Start(data.data_resource.offset))
                            //         .unwrap();

                            //     let cubemap_volume: Unk80806b7f = cur.read_le().unwrap();
                            //     let extents_center = Vec4::new(
                            //         data.translation.x,
                            //         data.translation.y,
                            //         data.translation.z,
                            //         data.translation.w,
                            //     );
                            //     let extents = Vec4::new(
                            //         cubemap_volume.cubemap_extents.x,
                            //         cubemap_volume.cubemap_extents.y,
                            //         cubemap_volume.cubemap_extents.z,
                            //         cubemap_volume.cubemap_extents.w,
                            //     );

                            //     let volume_min = extents_center - extents;
                            //     let volume_max = extents_center + extents;

                            //     renderer
                            //         .render_data
                            //         .load_texture(cubemap_volume.cubemap_texture);

                            //     resource_points.push(ResourcePoint {
                            //         translation: extents_center,
                            //         rotation: Quat::from_xyzw(
                            //             data.rotation.x,
                            //             data.rotation.y,
                            //             data.rotation.z,
                            //             data.rotation.w,
                            //         ),
                            //         entity: data.entity,
                            //         resource_type: data.data_resource.resource_type,
                            //         resource: MapResource::CubemapVolume(
                            //             Box::new(cubemap_volume),
                            //             AABB {
                            //                 min: volume_min.truncate().into(),
                            //                 max: volume_max.truncate().into(),
                            //             },
                            //         ),
                            //     });
                            // }
                            // // Point light
                            // 0x80806cbf => {
                            //     cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                            //         .unwrap();
                            //     let tag: TagHash = cur.read_le().unwrap();
                            //     resource_points.push(ResourcePoint {
                            //         translation: Vec4::new(
                            //             data.translation.x,
                            //             data.translation.y,
                            //             data.translation.z,
                            //             data.translation.w,
                            //         ),
                            //         rotation: Quat::from_xyzw(
                            //             data.rotation.x,
                            //             data.rotation.y,
                            //             data.rotation.z,
                            //             data.rotation.w,
                            //         ),
                            //         entity: data.entity,
                            //         resource_type: data.data_resource.resource_type,
                            //         resource: MapResource::PointLight(tag),
                            //     });
                            //     point_lights.push(Vec4::new(
                            //         data.translation.x,
                            //         data.translation.y,
                            //         data.translation.z,
                            //         data.translation.w,
                            //     ));
                            // }
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
            "Map {:x?} '{map_name}' - {} placement groups, {} decals",
            think.map_name,
            placement_groups.len(),
            resource_points
                .iter()
                .filter(|r| r.resource.is_decal())
                .count()
        );

        maps.push(MapData {
            hash: (package.pkg_id(), index as _).into(),
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
        })
    }

    // let to_load_entities: IntSet<TagHash> = maps
    //     .iter()
    //     .flat_map(|v| v.resource_points.iter().map(|(r, _)| r.entity))
    //     .filter(|v| v.is_valid())
    //     .collect();

    // let mut entity_renderers: IntMap<TagHash, EntityRenderer> = Default::default();
    // for te in &to_load_entities {
    //     let _span = debug_span!("Load entity", hash = %te).entered();
    //     let header: Unk80809c0f = package_manager().read_tag_struct(*te)?;
    //     debug!("Loading entity {te}");
    //     for e in &header.unk10 {
    //         match e.unk0.unk10.resource_type {
    //             0x808072b8 => {
    //                 debug!(
    //                     "\t- EntityModel {:08x}/{}",
    //                     e.unk0.unk18.resource_type.to_be(),
    //                     e.unk0.unk10.resource_type.to_be(),
    //                 );
    //                 let mut cur = Cursor::new(package_manager().read_tag(e.unk0.tag())?);
    //                 cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x1dc))?;
    //                 let model: Tag<Unk808073a5> = cur.read_le()?;
    //                 cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x300))?;
    //                 let entity_material_map: TablePointer<Unk808072c5> = cur.read_le()?;
    //                 let materials: TablePointer<Tag<Unk808071e8>> = cur.read_le()?;

    //                 for m in &materials {
    //                     material_map.insert(
    //                         m.tag(),
    //                         Material::load(&renderer, m.0.clone(), m.tag(), true),
    //                     );
    //                 }

    //                 for m in &model.meshes {
    //                     for p in &m.parts {
    //                         if p.material.is_valid() {
    //                             material_map.insert(
    //                                 p.material,
    //                                 Material::load(
    //                                     &renderer,
    //                                     package_manager().read_tag_struct(p.material)?,
    //                                     p.material,
    //                                     true,
    //                                 ),
    //                             );
    //                         }
    //                     }
    //                 }

    //                 if entity_renderers
    //                     .insert(
    //                         *te,
    //                         EntityRenderer::load(
    //                             model.0,
    //                             entity_material_map.to_vec(),
    //                             materials.iter().map(|m| m.tag()).collect_vec(),
    //                             &renderer,
    //                             &dcs,
    //                         )?,
    //                     )
    //                     .is_some()
    //                 {
    //                     error!("More than 1 model was loaded for entity {te}");
    //                 }

    //                 // println!(" - EntityModel {model:?}");
    //             }
    //             u => debug!(
    //                 "\t- Unknown entity resource type {:08X}/{:08X} (table {})",
    //                 u.to_be(),
    //                 e.unk0.unk10.resource_type.to_be(),
    //                 e.unk0.tag()
    //             ),
    //         }
    //     }

    //     if !entity_renderers.contains_key(te) {
    //         warn!("Entity {te} does not contain any geometry!");
    //     }
    // }

    // info!(
    //     "Found {} entity models ({} entities)",
    //     entity_renderers.len(),
    //     to_load_entities.len()
    // );

    info!("{} lights", point_lights.len());

    // // TODO(cohae): Maybe not the best idea?
    // info!("Updating resource constant buffers");
    // for m in &maps {
    //     for (rp, cb) in &m.resource_points {
    //         if let Some(ent) = entity_renderers.get(&rp.entity) {
    //             let mm = Mat4::from_scale_rotation_translation(
    //                 Vec3::splat(rp.translation.w),
    //                 rp.rotation.inverse(),
    //                 Vec3::ZERO,
    //             );
    //             let model_matrix = Mat4::from_cols(
    //                 mm.x_axis.truncate().extend(rp.translation.x),
    //                 mm.y_axis.truncate().extend(rp.translation.y),
    //                 mm.z_axis.truncate().extend(rp.translation.z),
    //                 mm.w_axis,
    //             );

    //             cb.write(&ScopeRigidModel {
    //                 mesh_to_world: model_matrix.transpose(),
    //                 position_scale: ent.mesh_scale(),
    //                 position_offset: ent.mesh_offset(),
    //                 texcoord0_scale_offset: ent.texcoord_transform(),
    //                 dynamic_sh_ao_values: Vec4::new(1.0, 1.0, 1.0, 0.0),
    //             })?;
    //         }
    //     }
    // }

    let mut placement_groups: IntMap<u32, (Unk8080966d, Vec<InstancedRenderer>)> =
        IntMap::default();

    let mut to_load: HashMap<TagHash, ()> = Default::default();
    let mut to_load_samplers: HashSet<ExtendedHash> = Default::default();
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
    info!("Loading {} terrain renderers", terrain_headers.len());
    info_span!("Loading terrain").in_scope(|| {
        for (t, header) in terrain_headers.into_iter() {
            for t in &header.mesh_groups {
                renderer
                    .render_data
                    .load_texture(ExtendedHash::Hash32(t.dyemap));
            }

            match TerrainRenderer::load(header, dcs.clone(), &renderer) {
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

    info!("Loading statics");
    info_span!("Loading statics").in_scope(|| {
        for almostloadable in &to_load_statics {
            let mheader: Unk808071a7 = package_manager().read_tag_struct(*almostloadable).unwrap();
            for m in &mheader.materials {
                if m.is_valid() {
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
                if m.is_valid() {
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

            match StaticModel::load(mheader, &renderer, *almostloadable) {
                Ok(model) => {
                    static_map.insert(*almostloadable, Arc::new(model));
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

    for m in material_map.values() {
        for t in m.ps_textures.iter().chain(m.vs_textures.iter()) {
            renderer.render_data.load_texture(t.texture);
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

    let cb_composite_lights =
        ConstantBuffer::<Vec4>::create_array_init(dcs.clone(), &point_lights)?;

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
    resources.insert(CurrentCubemap(None, None));
    resources.insert(ErrorRenderer::load(dcs.clone()));
    resources.insert(ScopeOverrides::default());
    resources.insert(DebugShapes::default());

    let _blend_state = unsafe {
        dcs.device.CreateBlendState(&D3D11_BLEND_DESC {
            RenderTarget: [D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: false.into(),
                SrcBlend: D3D11_BLEND_ONE,
                DestBlend: D3D11_BLEND_ZERO,
                BlendOp: D3D11_BLEND_OP_ADD,
                SrcBlendAlpha: D3D11_BLEND_ONE,
                DestBlendAlpha: D3D11_BLEND_ZERO,
                BlendOpAlpha: D3D11_BLEND_OP_ADD,
                RenderTargetWriteMask: (D3D11_COLOR_WRITE_ENABLE_RED.0
                    | D3D11_COLOR_WRITE_ENABLE_BLUE.0
                    | D3D11_COLOR_WRITE_ENABLE_GREEN.0)
                    as u8,
            }; 8],
            ..Default::default()
        })?
    };

    let gui_fps = Rc::new(RefCell::new(FpsDisplayOverlay::default()));
    let gui_rendersettings = Rc::new(RefCell::new(RenderSettingsOverlay {
        composition_mode: CompositorMode::Combined as usize,
        renderlayer_statics: true,
        renderlayer_statics_transparent: true,
        renderlayer_terrain: true,
        renderlayer_entities: false,

        alpha_blending: true,
        render_lights: false,
        blend_override: 0,
        evaluate_bytecode: false,
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
    }));

    let gui_resources = Rc::new(RefCell::new(ResourceTypeOverlay {
        debug_overlay: gui_debug.clone(),
    }));

    let gui_dump = Rc::new(RefCell::new(TagDumper::new()));
    let gui_loading = Rc::new(RefCell::new(LoadIndicatorOverlay::default()));

    let mut gui = GuiManager::create(&window, &dcs.device);
    let gui_console = Rc::new(RefCell::new(ConsoleOverlay::default()));
    gui.add_overlay(gui_debug);
    gui.add_overlay(gui_rendersettings.clone());
    gui.add_overlay(gui_resources);
    gui.add_overlay(gui_console);
    gui.add_overlay(gui_dump);
    gui.add_overlay(gui_loading);
    gui.add_overlay(gui_fps);

    {
        let mut data = renderer.render_data.data_mut();
        data.materials = material_map;
        data.vshaders = vshader_map;
        data.pshaders = pshader_map;
        data.samplers = sampler_map;
    };

    let _start_time = Instant::now();
    let mut last_frame = Instant::now();
    let mut last_cursor_pos: Option<PhysicalPosition<f64>> = None;
    let mut present_parameters = 0;

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

                    dcs.context()
                        .OMSetRenderTargets(Some(&[Some(new_rtv.clone())]), None);

                    *dcs.swapchain_target.write() = Some(new_rtv);

                    renderer
                        .resize((new_dims.width, new_dims.height))
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
                if !gui.imgui.io().want_capture_keyboard {
                    let mut camera = resources.get_mut::<FpsCamera>().unwrap();
                    let input_state = resources.get::<InputState>().unwrap();
                    camera.update(&input_state, last_frame.elapsed().as_secs_f32());
                }
                last_frame = Instant::now();

                let window_dims = window.inner_size();

                unsafe {
                    renderer.clear_render_targets();

                    dcs.context().RSSetViewports(Some(&[D3D11_VIEWPORT {
                        TopLeftX: 0.0,
                        TopLeftY: 0.0,
                        Width: window_dims.width as f32,
                        Height: window_dims.height as f32,
                        MinDepth: 0.0,
                        MaxDepth: 1.0,
                    }]));

                    dcs.context().RSSetState(&rasterizer_state);

                    renderer.begin_frame();

                    let maps = resources.get::<MapDataList>().unwrap();
                    let map = &maps.maps[maps.current_map % maps.maps.len()];

                    {
                        let gb = gui_rendersettings.borrow();

                        for ptag in &map.placement_groups {
                            let (_placements, instance_renderers) =
                                &placement_groups[&ptag.tag().0];
                            for instance in instance_renderers.iter() {
                                if gb.renderlayer_statics {
                                    instance.draw(&mut renderer, false).unwrap();
                                }

                                if gui_rendersettings.borrow().renderlayer_statics_transparent {
                                    instance.draw(&mut renderer, true).unwrap();
                                }
                            }
                        }

                        if gb.renderlayer_terrain {
                            for th in &map.terrains {
                                if let Some(t) = terrain_renderers.get(&th.0) {
                                    t.draw(&mut renderer).unwrap();
                                }
                            }
                        }

                        // if gb.renderlayer_entities {
                        //     for (rp, cb) in &map.resource_points {
                        //         if let Some(ent) = entity_renderers.get(&rp.entity) {
                        //             if ent.draw(&mut renderer, cb.buffer().clone()).is_err() {
                        //                 // resources.get::<ErrorRenderer>().unwrap().draw(
                        //                 //     &mut renderer,
                        //                 //     cb.buffer(),
                        //                 //     proj_view,
                        //                 //     view,
                        //                 // );
                        //             }
                        //         } else if rp.resource.is_entity() {
                        //             // resources.get::<ErrorRenderer>().unwrap().draw(
                        //             //     &mut renderer,
                        //             //     cb.buffer(),
                        //             //     proj_view,
                        //             //     view,
                        //             // );
                        //         }
                        //     }
                        // }
                    }

                    renderer.submit_frame(
                        &resources,
                        gui_rendersettings.borrow().render_lights,
                        gui_rendersettings.borrow().alpha_blending,
                        gui_rendersettings.borrow().composition_mode,
                        gui_rendersettings.borrow().blend_override,
                        (cb_composite_lights.buffer().clone(), point_lights.len()),
                        gui_rendersettings.borrow().evaluate_bytecode,
                    );

                    // let camera = resources.get::<FpsCamera>().unwrap();
                    // if let Some(MapResource::CubemapVolume(c, _)) = map
                    //     .resource_points
                    //     .iter()
                    //     .find(|(r, _)| {
                    //         if let MapResource::CubemapVolume(_, aabb) = &r.resource {
                    //             aabb.contains_point(camera.position)
                    //         } else {
                    //             false
                    //         }
                    //     })
                    //     .map(|(r, _)| &r.resource)
                    // {
                    //     if let Some(mut cr) = resources.get_mut::<CurrentCubemap>() {
                    //         cr.0 = Some(c.cubemap_name.to_string());
                    //     }
                    //     renderer
                    //         .render_data
                    //         .data()
                    //         .textures
                    //         .get(&c.cubemap_texture)
                    //         .map(|t| t.view.clone())
                    // } else {
                    //     if let Some(mut cr) = resources.get_mut::<CurrentCubemap>() {
                    //         cr.0 = None;
                    //     }
                    //     None
                    // };

                    // drop(camera);
                    drop(maps);
                    gui.draw_frame(&window, last_frame.elapsed(), &mut resources);

                    dcs.context().OMSetDepthStencilState(None, 0);

                    if dcs
                        .swap_chain
                        .Present(DXGI_SWAP_EFFECT_SEQUENTIAL.0 as _, present_parameters)
                        == DXGI_STATUS_OCCLUDED
                    {
                        present_parameters = DXGI_PRESENT_TEST;
                        std::thread::sleep(Duration::from_millis(50));
                    } else {
                        present_parameters = 0;
                    }

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
