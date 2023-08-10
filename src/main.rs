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
use nohash_hasher::IntMap;

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
use winit::event::{ElementState, MouseButton, VirtualKeyCode};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use crate::camera::{FpsCamera, InputState};
use crate::dxbc::{get_input_signature, DxbcHeader, DxbcInputType};
use crate::dxgi::calculate_pitch;
use crate::map::{Unk80806ef4, Unk8080714f, Unk80807dae, Unk80808a54, Unk808091e0, Unk808099d6};
use crate::map_resources::{MapResource, Unk80806b7f, Unk8080714b};
use crate::overlays::camera_settings::CameraPositionOverlay;
use crate::overlays::fps_display::FpsDisplayOverlay;
use crate::overlays::gbuffer_viewer::{
    CompositorMode, CompositorOptions, GBufferInfoOverlay, COMPOSITOR_MODES,
};
use crate::overlays::gui::GuiManager;
use crate::overlays::resource_nametags::{ResourcePoint, ResourceTypeOverlay};
use crate::packages::{package_manager, PACKAGE_MANAGER};
use crate::render::{DeviceContextSwapchain, GBuffer};
use crate::static_render::{LoadedTexture, StaticModel};
use crate::statics::{Unk808071a7, Unk8080966d};
use crate::terrain::TerrainRenderer;
use crate::text::{decode_text, StringData, StringPart, StringSetHeader};
use crate::texture::{TextureHandle, TextureHeader};
use crate::vertex_layout::InputElement;
use render::scopes::ScopeView;

mod camera;
mod dds;
mod dxbc;
mod dxgi;
mod entity;
mod icons;
mod map;
mod map_data;
mod map_resources;
mod material;
mod overlays;
mod packages;
mod render;
mod static_render;
mod statics;
mod structure;
mod terrain;
mod text;
mod texture;
mod types;
mod unknown;
mod vertex_layout;

// #[global_allocator]
// static GLOBAL: ProfiledAllocator<std::alloc::System> =
//     ProfiledAllocator::new(std::alloc::System, 100);

pub fn main() -> anyhow::Result<()> {
    rayon::ThreadPoolBuilder::new()
        .thread_name(|i| format!("rayon-worker-{i}"))
        .build_global()
        .unwrap();

    if cfg!(feature = "tracy") {
        tracing::subscriber::set_global_default(
            tracing_subscriber::registry()
                .with(tracing_tracy::TracyLayer::new())
                .with(tracing_subscriber::fmt::layer())
                .with(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::INFO.into())
                        .from_env_lossy(),
                ),
        )
    } else {
        tracing::subscriber::set_global_default(
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer())
                .with(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::INFO.into())
                        .from_env_lossy(),
                ),
        )
    }
    .expect("Failed to set up the tracing subscriber");

    let (package, mut pm) = info_span!("Initializing package manager").in_scope(|| {
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

    PACKAGE_MANAGER.with(|v| *v.borrow_mut() = Some(Arc::new(pm)));

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
        .with_inner_size(PhysicalSize::new(1600u32, 900u32))
        .build(&event_loop)?;

    // cohae: Slight concern for thread safety here. ID3D11Device is threadsafe, but ID3D11DeviceContext is *not*
    let dcs = Arc::new(DeviceContextSwapchain::create(&window)?);
    let mut gbuffer = GBuffer::create(
        (window.inner_size().width, window.inner_size().height),
        dcs.clone(),
    )?;

    let mut static_map: IntMap<u32, StaticModel> = Default::default();
    let mut material_map: IntMap<u32, material::Unk808071e8> = Default::default();
    let mut vshader_map: IntMap<u32, (ID3D11VertexShader, ID3D11InputLayout)> = Default::default();
    let mut pshader_map: IntMap<u32, ID3D11PixelShader> = Default::default();
    let mut cbuffer_map_vs: IntMap<u32, ID3D11Buffer> = Default::default();
    let mut cbuffer_map_ps: IntMap<u32, ID3D11Buffer> = Default::default();
    let mut texture_map: IntMap<u32, LoadedTexture> = Default::default();
    let mut sampler_map: IntMap<u32, ID3D11SamplerState> = Default::default();
    let mut terrain_headers = vec![];
    let mut maps: Vec<(u32, String, Vec<TagHash>, Vec<ResourcePoint>, Vec<TagHash>)> = vec![];
    for (index, _) in package.get_all_by_reference(0x80807dae) {
        let think: Unk80807dae = package_manager()
            .read_tag_struct((package.pkg_id(), index as _))
            .unwrap();
        let child_map: Unk808091e0 = package_manager()
            .read_tag_struct(think.child_map_reference)
            .unwrap();

        let mut placement_group_tags = vec![];
        let mut resource_points = vec![];
        let mut terrains = vec![];
        for res in &child_map.map_resources {
            let thing2: Unk80808a54 = if res.is_hash32 != 0 {
                package_manager().read_tag_struct(res.hash32).unwrap()
            } else {
                // TODO: Move TagHash64 to destiny-pkg
                package_manager().read_hash_struct(res.hash64.0).unwrap()
            };

            for tablehash in &thing2.data_tables {
                let table_data = package_manager().read_tag(*tablehash).unwrap();
                let mut cur = Cursor::new(&table_data);
                let table: Unk808099d6 = cur.read_le().unwrap();

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

                                placement_group_tags.push(preheader.placement_group);
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
                                    let mat: material::Unk808071e8 =
                                        package_manager().read_tag_struct(p.material).unwrap();
                                    material_map.insert(p.material.0, mat);
                                }

                                terrain_headers.push((terrain_resource.terrain, terrain));
                                terrains.push(terrain_resource.terrain);
                            }
                            // Cubemap volume
                            0x80806b7f => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset))
                                    .unwrap();

                                let cubemap_volume: Unk80806b7f = cur.read_le().unwrap();
                                resource_points.push(ResourcePoint {
                                    position: Vec4::new(
                                        data.translation.x,
                                        data.translation.y,
                                        data.translation.z,
                                        data.translation.w,
                                    ),
                                    resource_type: data.data_resource.resource_type,
                                    resource: MapResource::CubemapVolume(cubemap_volume),
                                });
                            }
                            // Point light
                            0x80806cbf => {
                                cur.seek(SeekFrom::Start(data.data_resource.offset + 16))
                                    .unwrap();
                                let tag: TagHash = cur.read_le().unwrap();
                                resource_points.push(ResourcePoint {
                                    position: Vec4::new(
                                        data.translation.x,
                                        data.translation.y,
                                        data.translation.z,
                                        data.translation.w,
                                    ),
                                    resource_type: data.data_resource.resource_type,
                                    resource: MapResource::PointLight(tag),
                                });
                            }
                            u => {
                                debug!(
                                    "Skipping unknown resource type {u:x} {:?}",
                                    data.translation
                                );
                                resource_points.push(ResourcePoint {
                                    position: Vec4::new(
                                        data.translation.x,
                                        data.translation.y,
                                        data.translation.z,
                                        data.translation.w,
                                    ),
                                    resource_type: data.data_resource.resource_type,
                                    resource: MapResource::Unknown(
                                        data.data_resource.resource_type,
                                    ),
                                });
                            }
                        };
                    } else {
                        resource_points.push(ResourcePoint {
                            position: Vec4::new(
                                data.translation.x,
                                data.translation.y,
                                data.translation.z,
                                data.translation.w,
                            ),
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
            placement_group_tags.len()
        ); // TODO(cohae): Map name lookup

        maps.push((
            think.map_name.0,
            map_name,
            placement_group_tags,
            resource_points,
            terrains,
        ));
    }

    let mut placement_groups: IntMap<u32, Unk8080966d> = IntMap::default();

    let mut to_load: HashMap<TagHash, ()> = Default::default();
    let mut to_load_textures: HashMap<TagHash, ()> = Default::default();
    let mut to_load_samplers: HashMap<TagHash, ()> = Default::default();
    for (_, _, placement_group_tags, _, _) in &maps {
        for tag in placement_group_tags.iter() {
            let placements: Unk8080966d = package_manager().read_tag_struct(*tag).unwrap();

            for v in &placements.statics {
                to_load.insert(*v, ());
            }

            placement_groups.insert(tag.0, placements);
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
                let mat: material::Unk808071e8 = package_manager().read_tag_struct(*m).unwrap();
                material_map.insert(m.0, mat);
            }

            match StaticModel::load(mheader, &dcs.device) {
                Ok(model) => {
                    static_map.insert(almostloadable.0, model);
                }
                Err(e) => {
                    error!(model = ?almostloadable, "Failed to load model: {e}");
                }
            }
        }
    });

    info!("Loaded {} statics", static_map.len());

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

                    let layout = vertex_layout::build_input_layout(
                        &input_sig
                            .elements
                            .iter()
                            .map(|e| {
                                InputElement::from_dxbc(
                                    e,
                                    e.component_type == DxbcInputType::Float,
                                    false,
                                )
                            })
                            .collect::<Vec<InputElement>>(),
                    );
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

                        let input_layout = dcs
                            .device
                            .CreateInputLayout(&layout, &vs_data)
                            .expect("Failed to create input layout");

                        (v, input_layout)
                    }
                });
            }

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
                let buf = unsafe {
                    dcs.device
                        .CreateBuffer(
                            &D3D11_BUFFER_DESC {
                                Usage: D3D11_USAGE_DYNAMIC,
                                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                                ByteWidth: (m.unk98.len() * std::mem::size_of::<Vec4>()) as _,
                                ..Default::default()
                            },
                            Some(&D3D11_SUBRESOURCE_DATA {
                                pSysMem: m.unk98.as_ptr() as _,
                                ..Default::default()
                            }),
                        )
                        .context("Failed to load float4 cbuffer")
                        .unwrap()
                };

                cbuffer_map_vs.insert(*t, buf);
            }

            if m.unk34c.is_valid() {
                let buffer_header_ref = package_manager().get_entry(m.unk34c).unwrap().reference;

                let buffer = package_manager().read_tag(buffer_header_ref).unwrap();
                trace!(
                    "Read {} bytes cbuffer from {buffer_header_ref:?}",
                    buffer.len()
                );
                let buf = unsafe {
                    dcs.device
                        .CreateBuffer(
                            &D3D11_BUFFER_DESC {
                                Usage: D3D11_USAGE_DYNAMIC,
                                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                                ByteWidth: buffer.len() as _,
                                ..Default::default()
                            },
                            Some(&D3D11_SUBRESOURCE_DATA {
                                pSysMem: buffer.as_ptr() as _,
                                ..Default::default()
                            }),
                        )
                        .context("Failed to load variable cbuffer")
                        .unwrap()
                };

                cbuffer_map_ps.insert(*t, buf);
            } else if !m.unk318.is_empty()
                && m.unk318
                    .iter()
                    .any(|v| v.x != 0.0 || v.y != 0.0 || v.z != 0.0 || v.w != 0.0)
            {
                trace!("Loading float4 cbuffer with {} elements", m.unk318.len());
                let buf = unsafe {
                    dcs.device
                        .CreateBuffer(
                            &D3D11_BUFFER_DESC {
                                Usage: D3D11_USAGE_DYNAMIC,
                                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                                ByteWidth: (m.unk318.len() * std::mem::size_of::<Vec4>()) as _,
                                ..Default::default()
                            },
                            Some(&D3D11_SUBRESOURCE_DATA {
                                pSysMem: m.unk318.as_ptr() as _,
                                ..Default::default()
                            }),
                        )
                        .context("Failed to load float4 cbuffer")
                        .unwrap()
                };

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

            let texture_header_ref = package_manager().get_entry(tex_hash).unwrap().reference;

            let texture: TextureHeader = package_manager().read_tag_struct(tex_hash).unwrap();
            let mut texture_data = if let Some(t) = texture.large_buffer {
                package_manager()
                    .read_tag(t)
                    .expect("Failed to read texture data")
            } else {
                package_manager()
                    .read_tag(texture_header_ref)
                    .expect("Failed to read texture data")
                    .to_vec()
            };

            let mut mips = 1;
            if texture.large_buffer.is_some() {
                let ab = package_manager()
                    .read_tag(texture_header_ref)
                    .expect("Failed to read texture data")
                    .to_vec();

                texture_data.extend(ab);

                let mut dim = texture.width.min(texture.width) as usize;
                mips = 0;
                while dim > 1 {
                    dim >>= 1;
                    mips += 1;
                }

                let mut required_mip_bytes = 0;
                for i in 0..mips {
                    let width = texture.width >> i;
                    let height = texture.height >> i;
                    let size = calculate_pitch(texture.format, width as usize, height as usize);
                    required_mip_bytes += size.1;
                }

                if required_mip_bytes > texture_data.len() {
                    warn!(
                        "Not enough bytes to satisfy {} mips (needed 0x{:x} bytes, got 0x{:x})",
                        mips,
                        required_mip_bytes,
                        texture_data.len()
                    );
                    mips = 1;
                }
            }

            let (tex, view) = unsafe {
                if texture.depth > 1 {
                    let (pitch, slice_pitch) =
                        calculate_pitch(texture.format, texture.width as _, texture.height as _);
                    let initial_data = D3D11_SUBRESOURCE_DATA {
                        pSysMem: texture_data.as_ptr() as _,
                        SysMemPitch: pitch as _,
                        SysMemSlicePitch: slice_pitch as _,
                    };

                    let tex = dcs
                        .device
                        .CreateTexture3D(
                            &D3D11_TEXTURE3D_DESC {
                                Width: texture.width as _,
                                Height: texture.height as _,
                                Depth: texture.depth as _,
                                MipLevels: 1,
                                Format: texture.format.into(),
                                Usage: D3D11_USAGE_DEFAULT,
                                BindFlags: D3D11_BIND_SHADER_RESOURCE,
                                CPUAccessFlags: Default::default(),
                                MiscFlags: Default::default(),
                            },
                            Some([initial_data].as_ptr()),
                        )
                        .context("Failed to create 3D texture")
                        .unwrap();

                    let view = dcs
                        .device
                        .CreateShaderResourceView(
                            &tex,
                            Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                                Format: texture.format.into(),
                                ViewDimension: D3D11_SRV_DIMENSION_TEXTURE3D,
                                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                                    Texture3D: D3D11_TEX3D_SRV {
                                        MostDetailedMip: 0,
                                        MipLevels: 1,
                                    },
                                },
                            }),
                        )
                        .unwrap();

                    (TextureHandle::Texture3D(tex), view)
                } else {
                    let mut initial_data = vec![];
                    let mut offset = 0;
                    for i in 0..mips {
                        let width = texture.width >> i;
                        let height = texture.height >> i;
                        let size = calculate_pitch(texture.format, width as usize, height as usize);

                        initial_data.push(D3D11_SUBRESOURCE_DATA {
                            pSysMem: texture_data.as_ptr().add(offset) as _,
                            SysMemPitch: size.0 as u32,
                            SysMemSlicePitch: 0,
                        });
                        offset += size.1;
                    }

                    let tex = dcs
                        .device
                        .CreateTexture2D(
                            &D3D11_TEXTURE2D_DESC {
                                Width: texture.width as _,
                                Height: texture.height as _,
                                MipLevels: mips as u32,
                                ArraySize: 1 as _,
                                Format: texture.format.into(),
                                SampleDesc: DXGI_SAMPLE_DESC {
                                    Count: 1,
                                    Quality: 0,
                                },
                                Usage: D3D11_USAGE_DEFAULT,
                                BindFlags: D3D11_BIND_SHADER_RESOURCE,
                                CPUAccessFlags: Default::default(),
                                MiscFlags: Default::default(),
                            },
                            Some(initial_data.as_ptr()),
                        )
                        .context("Failed to create texture")
                        .unwrap();

                    let view = dcs
                        .device
                        .CreateShaderResourceView(
                            &tex,
                            Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                                Format: texture.format.into(),
                                ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                                    Texture2D: D3D11_TEX2D_SRV {
                                        MostDetailedMip: 0,
                                        MipLevels: mips as _,
                                    },
                                },
                            }),
                        )
                        .unwrap();

                    (TextureHandle::Texture2D(tex), view)
                }
            };

            texture_map.insert(
                tex_hash.0,
                LoadedTexture {
                    handle: tex,
                    view,
                    format: texture.format,
                },
            );
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

    let _le_cbuffer = unsafe {
        dcs.device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: 2 * (4 * 4 * 4) + 4 * 4,
                ..Default::default()
            },
            None,
        )?
    };

    let _le_model_cbuffer = unsafe {
        dcs.device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: (4 * 4 * 4) * 3,
                ..Default::default()
            },
            None,
        )?
    };

    let le_model_cb0 = unsafe {
        dcs.device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: (64 * 4) * 4,
                ..Default::default()
            },
            None,
        )?
    };

    let le_vertex_cb11 = unsafe {
        dcs.device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: std::mem::size_of::<Mat4>() as _,
                ..Default::default()
            },
            None,
        )?
    };

    let le_vertex_cb12 = unsafe {
        dcs.device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: std::mem::size_of::<ScopeView>() as _,
                ..Default::default()
            },
            None,
        )?
    };

    let cb_composite_options = unsafe {
        dcs.device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: std::mem::size_of::<CompositorOptions> as _,
                ..Default::default()
            },
            None,
        )?
    };

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

    // let y_to_z_up: Mat4 = Mat4::from_rotation_x(-90f32.to_radians());

    let mut input_state = InputState {
        w: false,
        a: false,
        s: false,
        d: false,
        mouse1: false,
        shift: false,
        ctrl: false,
        space: false,
    };

    let camera: Rc<RefCell<FpsCamera>> = Rc::new(RefCell::new(FpsCamera::default()));

    unsafe {
        let cb11_data = vec![Vec4::splat(0.6); 128];
        let bmap = dcs
            .context
            .Map(&le_model_cb0, 0, D3D11_MAP_WRITE_DISCARD, 0)
            .context("Failed to map model cbuffer11")
            .unwrap();

        bmap.pData
            .copy_from_nonoverlapping(cb11_data.as_ptr() as _, std::mem::size_of::<Vec4>() * 64);

        dcs.context.Unmap(&le_model_cb0, 0);
    }

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

    let gui_fps = Rc::new(RefCell::new(FpsDisplayOverlay { delta: 0.0 }));
    let gui_gbuffer = Rc::new(RefCell::new(GBufferInfoOverlay {
        composition_mode: CompositorMode::Combined as usize,
        map_index: 0,
        maps: maps.clone(),
    }));
    let gui_debug = Rc::new(RefCell::new(CameraPositionOverlay {
        camera: camera.clone(),
        show_map_resources: false,
        show_unknown_map_resources: true,
        map_resource_distance: 2000.0,
        render_scale: 100.0,
        render_scale_changed: false,
    }));
    let gui_resources = Rc::new(RefCell::new(ResourceTypeOverlay {
        debug_overlay: gui_debug.clone(),
        map: (0, String::new(), vec![], vec![]),
    }));
    let mut gui_manager = GuiManager::create(&window, &dcs.device);
    gui_manager.add_overlay(Box::new(gui_fps.clone()));
    gui_manager.add_overlay(Box::new(gui_debug.clone()));
    gui_manager.add_overlay(Box::new(gui_gbuffer.clone()));
    gui_manager.add_overlay(Box::new(gui_resources.clone()));

    let _start_time = Instant::now();
    let mut last_frame = Instant::now();
    let mut last_cursor_pos: Option<PhysicalPosition<f64>> = None;

    event_loop.run(move |event, _, control_flow| {
        gui_manager.handle_event(&event, &window);
        match &event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::Resized(new_dims) => unsafe {
                        *dcs.swapchain_target.write().unwrap() = None;
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

                        dcs.context.OMSetRenderTargets(Some(&[Some(new_rtv.clone())]), None);

                        *dcs.swapchain_target.write().unwrap() = Some(new_rtv);

                        let render_scale = gui_debug.borrow().render_scale / 100.0;
                        gbuffer.resize(((new_dims.width as f32 * render_scale) as u32, (new_dims.height as f32 * render_scale) as u32)).expect("Failed to resize GBuffers");
                    },
                    WindowEvent::ScaleFactorChanged { .. } => {
                        // renderer.resize();
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if button == &MouseButton::Left && !gui_manager.imgui.io().want_capture_mouse {
                            input_state.mouse1 = *state == ElementState::Pressed
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(ref mut p) = last_cursor_pos {
                            let delta = (position.x - p.x, position.y - p.y);

                            if input_state.mouse1 && !gui_manager.imgui.io().want_capture_mouse {
                                camera.borrow_mut().update_mouse((delta.0 as f32, delta.1 as f32).into());
                            }

                            last_cursor_pos = Some(*position);
                        } else {
                            last_cursor_pos = Some(*position);
                        }
                    }
                    WindowEvent::ModifiersChanged(modifiers) => {
                        input_state.shift = modifiers.shift();
                        input_state.ctrl = modifiers.ctrl();
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        if input.state == ElementState::Pressed {
                            match input.virtual_keycode {
                                Some(VirtualKeyCode::Escape) => {
                                    *control_flow = ControlFlow::Exit
                                }
                                _ => {}
                            }
                        }

                        match input.virtual_keycode {
                            Some(VirtualKeyCode::W) => {
                                input_state.w = input.state == ElementState::Pressed
                            }
                            Some(VirtualKeyCode::A) => {
                                input_state.a = input.state == ElementState::Pressed
                            }
                            Some(VirtualKeyCode::S) => {
                                input_state.s = input.state == ElementState::Pressed
                            }
                            Some(VirtualKeyCode::D) => {
                                input_state.d = input.state == ElementState::Pressed
                            }
                            Some(VirtualKeyCode::Space) => {
                                input_state.space = input.state == ElementState::Pressed
                            }
                            _ => {}
                        }
                    }

                    _ => (),
                }
            }
            Event::RedrawRequested(..) => {
                let render_scale = gui_debug.borrow().render_scale / 100.0;
                if gui_debug.borrow().render_scale_changed {
                    let dims = window.inner_size();
                    gbuffer.resize(((dims.width as f32 * render_scale) as u32, (dims.height as f32 * render_scale) as u32)).expect("Failed to resize GBuffers");
                    // Just to be safe
                    gui_debug.borrow_mut().render_scale_changed = false;
                }

                camera.borrow_mut().update(&input_state, last_frame.elapsed().as_secs_f32());
                gui_fps.borrow_mut().delta = last_frame.elapsed().as_secs_f32();
                let window_dims = window.inner_size();
                last_frame = Instant::now();

                unsafe {
                    dcs.context.ClearRenderTargetView(&gbuffer.rt0.render_target, [0.0, 0.0, 0.0, 1.0].as_ptr() as _);
                    dcs.context.ClearRenderTargetView(&gbuffer.rt1.render_target, [0.0, 0.0, 0.0, 0.0].as_ptr() as _);
                    dcs.context.ClearRenderTargetView(&gbuffer.rt2.render_target, [0.0, 0.0, 0.0, 0.0].as_ptr() as _);
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
                        Some(&[Some(gbuffer.rt0.render_target.clone()), Some(gbuffer.rt1.render_target.clone()), Some(gbuffer.rt2.render_target.clone())]),
                        &gbuffer.depth.view,
                    );
                    dcs.context.OMSetDepthStencilState(
                        &gbuffer.depth.state,
                        0,
                    );

                    let projection = Mat4::perspective_infinite_reverse_rh(
                        90f32.to_radians(),
                        window_dims.width as f32 / window_dims.height as f32,
                        0.0001,
                    );

                    let view = camera.borrow_mut().calculate_matrix();

                    let bmap = &dcs.context
                        .Map(&le_vertex_cb12, 0, D3D11_MAP_WRITE_DISCARD, 0)
                        .unwrap();

                    let proj_view = projection * view;
                    let mut view2 = Mat4::IDENTITY;
                    view2.w_axis = camera.borrow_mut().position.extend(1.0);

                    let scope_view = ScopeView {
                        world_to_projective: proj_view,
                        camera_to_world: view2,
                        // Account for missing depth value in output
                        view_miscellaneous: Vec4::new(0.0, 0.0, 0.0001, 0.0),
                        ..Default::default()
                    };
                    bmap.pData.copy_from_nonoverlapping(
                        &scope_view as *const ScopeView as _,
                        std::mem::size_of::<ScopeView>(),
                    );

                    dcs.context.Unmap(&le_vertex_cb12, 0);

                    dcs.context.VSSetConstantBuffers(12, Some(&[Some(le_vertex_cb12.clone())]));

                    dcs.context.PSSetConstantBuffers(12, Some(&[Some(le_vertex_cb12.clone())]));

                    let map = &maps[gui_gbuffer.borrow().map_index % maps.len()];
                    gui_resources.borrow_mut().set_map_data(map.0, &map.1, map.2.clone(), map.3.clone());

                    for ptag in &map.2 {
                        let placements = &placement_groups[&ptag.0];
                        for instance in &placements.instances {
                            if let Some(model_hash) =
                                placements.statics.iter().nth(instance.static_index as _)
                            {
                                let _span =
                                    debug_span!("Draw static instance", count = instance.instance_count, model = ?model_hash)
                                        .entered();

                                if let Some(model) = static_map.get(&model_hash.0) {
                                    for transform in &placements.transforms[instance.instance_offset
                                        as usize
                                        ..(instance.instance_offset + instance.instance_count) as usize]
                                    {
                                        let mm = Mat4::from_scale_rotation_translation(
                                            Vec3::splat(transform.scale.x),
                                            Quat::from_xyzw(
                                                transform.rotation.x,
                                                transform.rotation.y,
                                                transform.rotation.z,
                                                transform.rotation.w,
                                            )
                                                .inverse(),
                                            Vec3::ZERO,
                                        );

                                        let model_matrix = Mat4::from_cols(
                                            mm.x_axis.truncate().extend(transform.translation.x),
                                            mm.y_axis.truncate().extend(transform.translation.y),
                                            mm.z_axis.truncate().extend(transform.translation.z),
                                            mm.w_axis,
                                        );

                                        let combined_matrix = model.mesh_transform() * model_matrix;

                                        let bmap = &dcs.context
                                            .Map(&le_vertex_cb11, 0, D3D11_MAP_WRITE_DISCARD, 0)
                                            .unwrap();

                                        let scope_instance = Mat4 {
                                            x_axis: combined_matrix.x_axis,
                                            y_axis: combined_matrix.y_axis,
                                            z_axis: combined_matrix.z_axis,
                                            w_axis: model
                                                .texcoord_transform()
                                                .extend(f32::from_bits(u32::MAX)),
                                        };

                                        bmap.pData.copy_from_nonoverlapping(
                                            &scope_instance as *const Mat4 as _,
                                            std::mem::size_of::<Mat4>(),
                                        );

                                        dcs.context.Unmap(&le_vertex_cb11, 0);
                                        dcs.context.VSSetConstantBuffers(
                                            11,
                                            Some(&[Some(le_vertex_cb11.clone())]),
                                        );

                                        model.draw(
                                            &dcs.context,
                                            &material_map,
                                            &vshader_map,
                                            &pshader_map,
                                            &cbuffer_map_vs,
                                            &cbuffer_map_ps,
                                            &texture_map,
                                            &sampler_map,
                                            le_model_cb0.clone(),
                                        );
                                    }
                                }
                            }
                        }
                    }

                    for th in &map.4 {
                        if let Some(t) = terrain_renderers.get(&th.0) {
                            t.draw(
                                &dcs.context,
                                &material_map,
                                &vshader_map,
                                &pshader_map,
                                &cbuffer_map_vs,
                                &cbuffer_map_ps,
                                &texture_map,
                                &sampler_map,
                                le_model_cb0.clone(),
                                &le_vertex_cb11,
                            );
                        }
                    }

                    dcs.context.OMSetRenderTargets(
                        Some(&[Some(dcs.swapchain_target.read().unwrap().as_ref().unwrap().clone())]),
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
                    let bmap = dcs.context
                        .Map(&cb_composite_options, 0, D3D11_MAP_WRITE_DISCARD, 0)
                        .unwrap();

                    let compositor_options = CompositorOptions {
                        proj_view_matrix_inv: proj_view.inverse(),
                        camera_pos: camera.borrow().position.extend(1.0),
                        camera_dir: camera.borrow().front.extend(1.0),
                        mode: COMPOSITOR_MODES[gui_gbuffer.borrow().composition_mode] as u32,
                    };
                    bmap.pData.cast::<CompositorOptions>().copy_from_nonoverlapping(
                        &compositor_options, 1
                    );

                    dcs.context.Unmap(&cb_composite_options, 0);
                    dcs.context
                        .PSSetConstantBuffers(0, Some(&[Some(cb_composite_options.clone())]));

                    dcs.context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                        TopLeftX: 0.0,
                        TopLeftY: 0.0,
                        Width: window_dims.width as f32,
                        Height: window_dims.height as f32,
                        MinDepth: 0.0,
                        MaxDepth: 1.0,
                    }]));

                    dcs.context.VSSetShader(&vshader_fullscreen, None);
                    dcs.context.PSSetShader(&pshader_fullscreen, None);
                    dcs.context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
                    dcs.context.Draw(3, 0);

                    gui_manager.draw_frame(&window, last_frame.elapsed());

                    dcs.context.OMSetDepthStencilState(None, 0);

                    dcs.swap_chain.Present(1, 0).unwrap();

                    tracy_client::Client::running().map(|c| c.frame_mark());
                };
            }
            Event::MainEventsCleared => {
                let io = gui_manager.imgui.io_mut();
                gui_manager.platform.prepare_frame(io, &window)
                    .expect("Failed to start frame");
                window.request_redraw();
            }
            _ => (),
        }
    });
}
