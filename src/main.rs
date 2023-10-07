#![warn(rust_2018_idioms)]
#![deny(clippy::correctness, clippy::suspicious, clippy::complexity)]

#[macro_use]
extern crate windows;

#[macro_use]
extern crate tracing;

use std::cell::RefCell;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::mem::transmute;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::overlays::console::ConsoleOverlay;
use crate::util::{FilterDebugLockTarget, RwLock};
use anyhow::Context;
use binrw::BinReaderExt;
use clap::Parser;
use destiny_pkg::PackageVersion::{self};
use destiny_pkg::{PackageManager, TagHash};
use itertools::Itertools;
use nohash_hasher::IntMap;
use poll_promise::Promise;
use strum::EnumCount;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Layer};
use windows::Win32::Foundation::DXGI_STATUS_OCCLUDED;
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
use crate::input::InputState;
use crate::map::MapDataList;
use crate::map_resources::MapResource;
use crate::mapload_temporary::load_maps;
use crate::overlays::camera_settings::CameraPositionOverlay;

use crate::overlays::fps_display::FpsDisplayOverlay;
use crate::overlays::gui::GuiManager;
use crate::overlays::load_indicator::LoadIndicatorOverlay;
use crate::overlays::render_settings::{RenderSettings, RenderSettingsOverlay};
use crate::overlays::resource_nametags::ResourceTypeOverlay;
use crate::overlays::tag_dump::TagDumper;
use crate::packages::{package_manager, PACKAGE_MANAGER};
use crate::render::debug::DebugShapes;
use crate::render::error::ErrorRenderer;
use crate::render::overrides::{EnabledShaderOverrides, ScopeOverrides};
use crate::render::renderer::Renderer;

use crate::render::{DeviceContextSwapchain, EntityRenderer, InstancedRenderer, TerrainRenderer};
use crate::resources::Resources;

use crate::statics::Unk8080966d;
use crate::text::{decode_text, StringData, StringPart, StringSetHeader};

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
mod mapload_temporary;
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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, disable_version_flag(true))]
struct Args {
    /// Package to use
    package: String,

    /// Map hash to load. Ignores package argument
    #[arg(short, long)]
    map: Option<String>,
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    util::fix_windows_command_prompt();
    panic_handler::install_hook();

    #[cfg(not(debug_assertions))]
    std::env::set_var("RUST_BACKTRACE", "0");

    let args = Args::parse();

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
            .with(overlays::console::ConsoleLogLayer.with_filter(FilterDebugLockTarget))
            .with(tracing_subscriber::fmt::layer().with_filter(FilterDebugLockTarget))
            // .with(FilterDebugLockTarget)
            .with(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            ),
    )
    .expect("Failed to set up the tracing subscriber");

    let (package, pm) = info_span!("Initializing package manager").in_scope(|| {
        (
            PackageVersion::Destiny2Lightfall
                .open(&args.package)
                .expect("Failed to open package"),
            PackageManager::new(
                PathBuf::from_str(&args.package).unwrap().parent().unwrap(),
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

    let stringmap = Arc::new(stringmap);

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
    let window = Arc::new(window);

    let dcs = Arc::new(DeviceContextSwapchain::create(&window)?);

    // TODO(cohae): resources should be added to renderdata directly
    let renderer = Arc::new(RwLock::new(Renderer::create(&window, dcs.clone())?));

    let map_hashes = if let Some(map_hash) = args.map {
        let hash = match u32::from_str_radix(&map_hash, 16) {
            Ok(v) => TagHash(u32::from_be(v)),
            Err(_e) => anyhow::bail!("The given map '{map_hash}' is not a valid hash!"),
        };

        if package_manager()
            .get_entry(hash)
            .context("Could not find given map hash")?
            .reference
            != u32::from_be(0x1E898080)
        {
            anyhow::bail!("The given hash '{map_hash}' is not a map!")
        }

        vec![hash]
    } else {
        package
            .get_all_by_reference(u32::from_be(0x1E898080))
            .into_iter()
            .map(|(index, _entry)| TagHash::new(package.pkg_id(), index as u16))
            .collect_vec()
    };

    let mut map_load_task = Some(Promise::spawn_async(load_maps(
        dcs.clone(),
        renderer.clone(),
        map_hashes,
        stringmap.clone(),
    )));
    let mut entity_renderers: IntMap<u64, EntityRenderer> = Default::default();
    let mut placement_renderers: IntMap<u32, (Unk8080966d, Vec<InstancedRenderer>)> =
        IntMap::default();
    let mut terrain_renderers: IntMap<u32, TerrainRenderer> = Default::default();

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
        maps: vec![],
    });
    resources.insert(ErrorRenderer::load(dcs.clone()));
    resources.insert(ScopeOverrides::default());
    resources.insert(DebugShapes::default());
    resources.insert(EnabledShaderOverrides::default());
    resources.insert(RenderSettings::default());

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
        renderlayer_statics: true,
        renderlayer_statics_transparent: true,
        renderlayer_terrain: true,
        renderlayer_entities: true,
        renderlayer_background: true,
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

    let mut gui = GuiManager::create(&window, dcs.clone());
    let gui_console = Rc::new(RefCell::new(ConsoleOverlay::default()));
    gui.add_overlay(gui_debug);
    gui.add_overlay(gui_rendersettings.clone());
    gui.add_overlay(gui_resources);
    gui.add_overlay(gui_console);
    gui.add_overlay(gui_dump);
    gui.add_overlay(gui_loading);
    gui.add_overlay(gui_fps);

    let _start_time = Instant::now();
    let mut last_frame = Instant::now();
    let mut last_cursor_pos: Option<PhysicalPosition<f64>> = None;
    let mut present_parameters = 0;

    event_loop.run(move |event, _, control_flow| {
        match &event {
            Event::WindowEvent { event, .. } => {
                let gui_event_captured = gui.handle_event(event).consumed;

                if !gui_event_captured {
                    resources
                        .get_mut::<InputState>()
                        .unwrap()
                        .handle_event(event);
                }

                match event {
                    WindowEvent::Resized(new_dims) => unsafe {
                        let _ = gui
                            .renderer
                            .resize_buffers(transmute(&dcs.swap_chain), || {
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
                                    .write()
                                    .resize((new_dims.width, new_dims.height))
                                    .expect("Failed to resize GBuffers");

                                transmute(0i32)
                            })
                            .unwrap();
                    },
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(ref mut p) = last_cursor_pos {
                            let delta = (position.x - p.x, position.y - p.y);
                            let input = resources.get::<InputState>().unwrap();
                            if input.mouse_left() && !gui_event_captured {
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
                }
            }
            Event::RedrawRequested(..) => {
                // if !gui_event_captured
                {
                    let mut camera = resources.get_mut::<FpsCamera>().unwrap();
                    let input_state = resources.get::<InputState>().unwrap();
                    camera.update(&input_state, last_frame.elapsed().as_secs_f32());
                }
                last_frame = Instant::now();

                let window_dims = window.inner_size();

                if map_load_task.as_ref().and_then(|v| v.ready()).is_some() {
                    if let Some(Ok(map_res)) = map_load_task.take().map(|v| v.try_take()) {
                        let map_res = map_res.expect("Failed to load map(s)");
                        entity_renderers.extend(map_res.entity_renderers);
                        terrain_renderers.extend(map_res.terrain_renderers);
                        placement_renderers.extend(map_res.placement_renderers);
                        let mut maps = resources.get_mut::<MapDataList>().unwrap();
                        maps.maps = map_res.maps;
                        map_load_task = None;
                    }
                }

                unsafe {
                    renderer.read().clear_render_targets(
                        resources.get::<RenderSettings>().unwrap().clear_color,
                    );

                    dcs.context().RSSetViewports(Some(&[D3D11_VIEWPORT {
                        TopLeftX: 0.0,
                        TopLeftY: 0.0,
                        Width: window_dims.width as f32,
                        Height: window_dims.height as f32,
                        MinDepth: 0.0,
                        MaxDepth: 1.0,
                    }]));

                    dcs.context().RSSetState(&rasterizer_state);

                    renderer.read().begin_frame();

                    let maps = resources.get::<MapDataList>().unwrap();

                    let mut lights = None;
                    if !maps.maps.is_empty() {
                        let (_, _, map) = &maps.maps[maps.current_map % maps.maps.len()];
                        lights = Some((map.lights_cbuffer.buffer().clone(), map.lights.len()));

                        {
                            let gb = gui_rendersettings.borrow();

                            for ptag in &map.placement_groups {
                                let (_placements, instance_renderers) =
                                    &placement_renderers[&ptag.tag().0];
                                for instance in instance_renderers.iter() {
                                    if gb.renderlayer_statics {
                                        instance.draw(&renderer.read(), false).unwrap();
                                    }

                                    if gui_rendersettings.borrow().renderlayer_statics_transparent {
                                        instance.draw(&renderer.read(), true).unwrap();
                                    }
                                }
                            }

                            if gb.renderlayer_terrain {
                                for th in &map.terrains {
                                    if let Some(t) = terrain_renderers.get(&th.0) {
                                        t.draw(&renderer.read()).unwrap();
                                    }
                                }
                            }

                            for (rp, cb) in &map.resource_points {
                                match rp.resource {
                                    MapResource::Unk80806aa3 { .. } => {
                                        if !gb.renderlayer_background {
                                            continue;
                                        }
                                    }
                                    _ => {
                                        if !gb.renderlayer_entities {
                                            continue;
                                        }
                                    }
                                }

                                // if gb.renderlayer_entities {
                                //     // Veil roots
                                //     // if rp.entity.hash32() != Some(TagHash(u32::from_be(0x68e8e780))) {
                                //     //     continue;
                                //     // }

                                //     // Metaverse cat
                                //     // if rp.entity.hash32() != Some(TagHash(u32::from_be(0x2BF6E780))) {
                                //     //     continue;
                                //     // }

                                if let Some(ent) = entity_renderers.get(&rp.entity_key()) {
                                    if ent.draw(&renderer.read(), cb.buffer().clone()).is_err() {
                                        // resources.get::<ErrorRenderer>().unwrap().draw(
                                        //     &mut renderer,
                                        //     cb.buffer(),
                                        //     proj_view,
                                        //     view,
                                        // );
                                    }
                                } else if rp.resource.is_entity() {
                                    // resources.get::<ErrorRenderer>().unwrap().draw(
                                    //     &mut renderer,
                                    //     cb.buffer(),
                                    //     proj_view,
                                    //     view,
                                    // );
                                }
                            }
                        }

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
                    }
                    drop(maps);

                    renderer.read().submit_frame(&resources, lights);

                    gui.draw_frame(window.clone(), &mut resources, |ctx| {
                        if let Some(task) = map_load_task.as_ref() {
                            if task.ready().is_none() {
                                egui::Window::new("Loading...")
                                    .title_bar(false)
                                    .resizable(false)
                                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                                    .show(ctx, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spinner();
                                            ui.heading("Loading maps")
                                        })
                                    });
                            }
                        }
                    });

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
