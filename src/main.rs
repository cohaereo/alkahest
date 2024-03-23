#![warn(rust_2018_idioms)]
#![deny(clippy::correctness, clippy::suspicious, clippy::complexity)]
#![allow(clippy::collapsible_else_if)]

#[macro_use]
extern crate windows;

#[macro_use]
extern crate tracing;

use std::{
    cell::RefCell,
    f32::consts::PI,
    fmt::Write,
    io::Cursor,
    mem::transmute,
    path::PathBuf,
    rc::Rc,
    str::FromStr,
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

use alkahest_data::{render_globals::SRenderGlobals, tag::ExtendedHash};
use anyhow::Context;
use binrw::BinReaderExt;
use clap::Parser;
use destiny_pkg::{
    PackageManager,
    PackageVersion::{self},
    TagHash,
};
use dxbc::{get_input_signature, get_output_signature, DxbcHeader, DxbcInputType};
use ecs::{components::CubemapVolume, transform::Transform};
use egui::epaint::{ahash::HashMap, Hsva};
use glam::{Mat4, Quat, Vec3};
use hecs::Entity;
use itertools::Itertools;
use mimalloc::MiMalloc;
use overlays::camera_settings::CurrentCubemap;
use render::{color::Color, debug::DebugDrawFlags, vertex_layout::InputElement};
use technique::Technique;
use text::GlobalStringmap;
use tiger_parse::PackageManagerExt;
use tracing::level_filters::LevelFilter;
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Layer};
use windows::Win32::{
    Foundation::DXGI_STATUS_OCCLUDED,
    Graphics::{
        Direct3D11::*,
        Dxgi::{Common::*, DXGI_PRESENT_TEST, DXGI_SWAP_EFFECT_SEQUENTIAL},
    },
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::windows::WindowBuilderExtWindows,
};

use crate::{
    action::ActionList,
    camera::FpsCamera,
    config::{WindowConfig, CONFIGURATION},
    ecs::{
        components::{
            ActivityGroup, Beacon, EntityModel, ResourcePoint, Route, Ruler, Sphere,
            StaticInstances, Terrain, Visible, Water,
        },
        resolve_aabb,
        resources::SelectedEntity,
    },
    hotkeys::{SHORTCUT_FOCUS, SHORTCUT_GAZE, SHORTCUT_MAP_SWAP},
    input::InputState,
    map::{MapList, MapLoadState},
    map_resources::MapResource,
    overlays::{
        activity_select::{ActivityBrowser, CurrentActivity},
        camera_settings::CameraPositionOverlay,
        console::ConsoleOverlay,
        fps_display::FpsDisplayOverlay,
        gui::{GuiManager, HiddenWindows, PreDrawResult, ViewerWindows},
        inspector::InspectorOverlay,
        load_indicator::{LoadIndicator, LoadIndicatorOverlay, LoadIndicators},
        menu::MenuBar,
        outliner::OutlinerOverlay,
        render_settings::{ActivityGroupFilter, RenderSettings, RenderSettingsOverlay},
        resource_nametags::ResourceTypeOverlay,
        tag_dump::{BulkTextureDumper, TagDumper},
        updater::{ChannelSelector, UpdateDownload},
    },
    packages::{package_manager, PACKAGE_MANAGER},
    render::{
        debug::DebugShapes,
        overrides::{EnabledShaderOverrides, ScopeOverrides},
        renderer::{Renderer, RendererShared, ShadowMapsResource},
        tween::ease_out_exponential,
        DeviceContextSwapchain,
    },
    resources::Resources,
    texture::{Texture, LOW_RES},
    updater::UpdateCheck,
    util::{
        consts::{self, print_banner},
        image::Png,
        text::{invert_color, keep_color_bright, prettify_distance},
        FilterDebugLockTarget, RwLock,
    },
};

mod action;
mod camera;
mod config;
#[cfg(feature = "discord_rpc")]
mod discord;
mod dxbc;
mod ecs;
mod game_selector;
mod hotkeys;
mod icons;
mod input;
mod map;
mod map_resources;
mod mapload_temporary;
mod overlays;
mod packages;
mod render;
mod resources;
mod technique;
mod text;
mod texture;
mod types;
mod updater;
mod util;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, disable_version_flag(true))]
struct Args {
    /// Packages directory
    package_dir: Option<String>,

    /// Package prefix to load maps from, ignores package argument.
    /// For example: `throneworld`, `edz`
    #[arg(short, long)]
    package_name: Option<String>,

    /// Map hash to load. Ignores package argument(s)
    #[arg(short, long)]
    map: Option<String>,

    #[arg(short, long)]
    activity: Option<String>,

    #[arg(long, alias = "na")]
    no_ambient: bool,

    #[arg(long)]
    lowres: bool,
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    util::fix_windows_command_prompt();

    let mut panic_header = String::new();
    writeln!(&mut panic_header, "Alkahest v{}", consts::VERSION).unwrap();
    writeln!(&mut panic_header, "Built from commit {}", consts::GIT_HASH).unwrap();
    writeln!(&mut panic_header, "Built on {}", consts::BUILD_TIMESTAMP).unwrap();

    alkahest_panic_handler::install_hook(Some(panic_header));

    print_banner();

    config::load();

    let icon_data = Png::from_bytes(include_bytes!("../assets/icon.png"))?;
    let icon = winit::window::Icon::from_rgba(
        icon_data.data.to_vec(),
        icon_data.dimensions[0] as u32,
        icon_data.dimensions[1] as u32,
    )
    .unwrap();

    let args = Args::parse();

    LOW_RES.store(args.lowres, Ordering::Relaxed);

    rayon::ThreadPoolBuilder::new()
        .thread_name(|i| format!("rayon-worker-{i}"))
        .build_global()
        .unwrap();

    let tracy_layer = if cfg!(feature = "tracy") {
        Some(tracing_tracy::TracyLayer::default())
    } else {
        None
    };

    LogTracer::init()?;
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

    let mut event_loop = EventLoop::new();
    let package_dir = if let Some(p) = &args.package_dir {
        if p.ends_with(".pkg") {
            warn!("Please specify the directory containing the packages, not the package itself! Support for this will be removed in the future!");
            PathBuf::from_str(p)
                .context("Invalid package directory")?
                .parent()
                .unwrap()
                .to_path_buf()
        } else {
            PathBuf::from_str(p).context("Invalid package directory")?
        }
    } else if let Some(p) = config::with(|c| c.packages_directory.clone()) {
        PathBuf::from_str(&p).context("Invalid package directory")?
    } else {
        let path = PathBuf::from_str(
            &game_selector::select_game_installation(&mut event_loop, &icon)
                .context("No game installation selected")?,
        )
        .unwrap();

        path.join("packages")
    };

    if !package_dir.exists() {
        config::with_mut(|c| c.packages_directory = None);
        config::persist();

        panic!(
            "The specified package directory does not exist! ({})\nRelaunch alkahest with a valid package directory.",
            package_dir.display()
        );
    }

    let pm = info_span!("Initializing package manager")
        .in_scope(|| PackageManager::new(package_dir, PackageVersion::Destiny2Lightfall).unwrap());

    config::with_mut(|c| c.packages_directory = Some(pm.package_dir.to_string_lossy().to_string()));
    config::persist();

    *PACKAGE_MANAGER.write() = Some(Arc::new(pm));

    let window = winit::window::WindowBuilder::new()
        .with_title("Alkahest")
        .with_inner_size(config::with(|c| {
            PhysicalSize::new(c.window.width, c.window.height)
        }))
        .with_position(config::with(|c| {
            PhysicalPosition::new(c.window.pos_x, c.window.pos_y)
        }))
        .with_maximized(config!().window.maximised)
        .with_window_icon(Some(icon.clone()))
        .with_taskbar_icon(Some(icon))
        .build(&event_loop)?;
    let window = Arc::new(window);

    // #[cfg(not(debug_assertions))]
    // std::env::set_var("RUST_BACKTRACE", "0");

    let stringmap = Arc::new(GlobalStringmap::load().context("Failed to load global strings")?);

    info!("Loaded {} global strings", stringmap.0.len());

    let dcs = Arc::new(DeviceContextSwapchain::create(&window)?);

    // TODO(cohae): resources should be added to renderdata directly
    let renderer: RendererShared = Arc::new(RwLock::new(Renderer::create(&window, dcs.clone())?));

    load_render_globals(&renderer.read());

    let map_hashes = if let Some(map_hash) = &args.map {
        let hash = match u32::from_str_radix(map_hash, 16) {
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
    } else if let Some(package_name) = &args.package_name {
        package_manager()
            .get_all_by_reference(u32::from_be(0x1E898080))
            .into_iter()
            .filter(|(tag, _)| {
                package_manager().package_paths[&tag.pkg_id()]
                    .name
                    .to_lowercase()
                    .contains(package_name.as_str())
            })
            .map(|(tag, _entry)| tag)
            .collect_vec()
    } else {
        vec![]
    };

    let activity_hash = args.activity.as_ref().map(|a| {
        TagHash(u32::from_be(
            u32::from_str_radix(a, 16)
                .context("Invalid activity hash format")
                .unwrap(),
        ))
    });

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
    resources.insert(MapList::default());
    resources.insert(ScopeOverrides::default());
    resources.insert(DebugShapes::default());
    resources.insert(EnabledShaderOverrides::default());
    let mut render_settings = RenderSettings::default();
    render_settings.draw_crosshair = config::with(|cfg| cfg.render_settings.draw_crosshair);
    resources.insert(render_settings);
    resources.insert(ShadowMapsResource::create(dcs.clone()));
    resources.insert(CurrentCubemap(None, None));
    resources.insert(ActivityGroupFilter::default());
    resources.insert(ViewerWindows::default());
    resources.insert(renderer.clone());
    resources.insert(renderer.read().dcs.clone());
    resources.insert(SelectedEntity(None, false, Instant::now()));
    resources.insert(UpdateCheck::default());
    resources.insert(LoadIndicators::default());
    resources.insert(args.clone());
    resources.insert(Arc::clone(&stringmap));
    resources.insert(HiddenWindows::default());
    resources.insert(ActionList::default());
    resources.insert(CurrentActivity::default());

    let mut activity_browser = ActivityBrowser::new(&stringmap);

    if let Some(activity_hash) = &activity_hash {
        let mut maps = mapload_temporary::query_activity_maps(*activity_hash, &stringmap)?;
        if args.map.is_some() {
            maps.retain(|(hash, _)| map_hashes.contains(hash));
        }

        let mut map_list = resources.get_mut::<MapList>().unwrap();
        map_list.set_maps(&maps);
    } else {
        let mut maps = vec![];

        for hash in map_hashes {
            let name = mapload_temporary::get_map_name(hash, &stringmap)?;
            maps.push((hash, name));
        }

        let mut map_list = resources.get_mut::<MapList>().unwrap();
        map_list.set_maps(&maps);
    }
    // resources.get_mut::<MapList>().unwrap().load_all(&resources);

    let gui_fps = Rc::new(RefCell::new(FpsDisplayOverlay::default()));
    let gui_rendersettings = Rc::new(RefCell::new(RenderSettingsOverlay {
        renderlayer_statics: true,
        renderlayer_statics_transparent: true,
        renderlayer_statics_decals: true,
        renderlayer_terrain: true,
        renderlayer_entities: true,
        renderlayer_background: true,
        renderlayer_water: true,
        shadow_res_index: 1,
        animate_light: false,
        light_dir_degrees: Vec3::new(1.0, 0.0, 50.0),
        last_frame: Instant::now(),
    }));
    let gui_debug = Rc::new(RefCell::new(CameraPositionOverlay {
        show_map_resources: config::with(|cfg| cfg.resources.show_resources),
        show_map_resource_label: true,
        map_resource_label_background: config::with(|cfg| {
            cfg.resources.map_resource_label_background
        }),
        map_resource_filter: {
            let mut f = vec![false; MapResource::max_index() + 1];

            config::with(|cfg| {
                for (k, v) in cfg.resources.filters.iter() {
                    if let Some(index) = MapResource::id_to_index(k) {
                        f[index] = *v;
                    }
                }
            });

            f
        },
        map_resource_distance: 2000.0,
        map_resource_distance_limit_enabled: config::with(|cfg| {
            cfg.resources.resource_distance_limit
        }),
        map_resource_only_show_named: false,
        map_resource_show_activity: true,
        map_resource_show_map: true,
    }));

    let gui_resources = Rc::new(RefCell::new(ResourceTypeOverlay {
        debug_overlay: gui_debug.clone(),
    }));

    let gui_dump = Rc::new(RefCell::new(TagDumper::new()));
    let gui_loading = Rc::new(RefCell::new(LoadIndicatorOverlay));

    let mut gui = GuiManager::create(&window, dcs.clone());
    let gui_console = Rc::new(RefCell::new(ConsoleOverlay::default()));
    gui.add_overlay(gui_debug.clone());
    gui.add_overlay(gui_rendersettings.clone());
    gui.add_overlay(gui_resources);
    gui.add_overlay(gui_console);
    gui.add_overlay(gui_dump);
    gui.add_overlay(gui_loading);
    gui.add_overlay(gui_fps);

    gui.add_overlay(Rc::new(RefCell::new(InspectorOverlay)));
    gui.add_overlay(Rc::new(RefCell::new(OutlinerOverlay::default())));
    gui.add_overlay(Rc::new(RefCell::new(MenuBar::default())));
    gui.add_overlay(Rc::new(RefCell::new(BulkTextureDumper::default())));

    let mut update_channel_gui = ChannelSelector {
        open: config::with(|c| c.update_channel.is_none()),
    };

    let mut updater_gui: Option<UpdateDownload> = None;

    if let Some(update_channel) = config::with(|c| c.update_channel) {
        resources
            .get_mut::<UpdateCheck>()
            .unwrap()
            .start(update_channel);
    }

    let start_time = Instant::now();
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
                            if (input.mouse_left() | input.mouse_middle()) && !gui_event_captured {
                                let mut camera = resources.get_mut::<FpsCamera>().unwrap();
                                camera.update_mouse((delta.0 as f32, delta.1 as f32).into());

                                // Wrap the cursor around if it goes out of bounds
                                let window_dims = window.inner_size();
                                let window_dims =
                                    (window_dims.width as i32, window_dims.height as i32);
                                let cursor_pos = (position.x as i32, position.y as i32);
                                let mut new_cursor_pos = cursor_pos;

                                if cursor_pos.0 <= 0 {
                                    new_cursor_pos.0 = window_dims.0;
                                } else if cursor_pos.0 >= (window_dims.0 - 1) {
                                    new_cursor_pos.0 = 0;
                                }

                                if cursor_pos.1 <= 0 {
                                    new_cursor_pos.1 = window_dims.1;
                                } else if cursor_pos.1 >= window_dims.1 {
                                    new_cursor_pos.1 = 0;
                                }

                                if new_cursor_pos != cursor_pos {
                                    window
                                        .set_cursor_position(PhysicalPosition::new(
                                            new_cursor_pos.0 as f64,
                                            new_cursor_pos.1 as f64,
                                        ))
                                        .ok();
                                }
                                last_cursor_pos = Some(PhysicalPosition::new(
                                    new_cursor_pos.0 as f64,
                                    new_cursor_pos.1 as f64,
                                ));
                            } else {
                                last_cursor_pos = Some(*position);
                            }
                        } else {
                            last_cursor_pos = Some(*position);
                        }
                    }
                    // TODO(cohae): Should this even be in here at this point?
                    WindowEvent::KeyboardInput { .. } => {
                        let input = resources.get::<InputState>().unwrap();

                        if input.is_key_pressed(VirtualKeyCode::Up) {
                            if let Some(selected_entity) =
                                resources.get_mut::<SelectedEntity>().unwrap().0.as_mut()
                            {
                                *selected_entity = Entity::from_bits(
                                    selected_entity.to_bits().get().saturating_add(1),
                                )
                                .unwrap_or(*selected_entity);
                            }
                        }

                        if input.is_key_pressed(VirtualKeyCode::Down) {
                            if let Some(selected_entity) =
                                resources.get_mut::<SelectedEntity>().unwrap().0.as_mut()
                            {
                                *selected_entity = Entity::from_bits(
                                    selected_entity.to_bits().get().saturating_sub(1),
                                )
                                .unwrap_or(*selected_entity);
                            }
                        }
                    }
                    _ => (),
                }
            }
            Event::RedrawRequested(..) => {
                {
                    let mut action_list = resources.get_mut::<ActionList>().unwrap();
                    action_list.process(&resources);
                }

                resources.get_mut::<SelectedEntity>().unwrap().1 = false;

                {
                    let mut maps = resources.get_mut::<MapList>().unwrap();
                    maps.update_maps(&resources);
                }

                // if !gui_event_captured
                {
                    let mut camera = resources.get_mut::<FpsCamera>().unwrap();
                    let input_state = resources.get::<InputState>().unwrap();
                    camera.update(
                        &input_state,
                        window.inner_size().into(),
                        last_frame.elapsed().as_secs_f32(),
                    );

                    if gui.egui.input_mut(|i| i.consume_shortcut(&SHORTCUT_FOCUS)) {
                        if let Some(selected_entity) = resources.get::<SelectedEntity>() {
                            let maps = resources.get::<MapList>().unwrap();

                            if let Some(map) = maps.current_map() {
                                if let Ok(e) = map
                                    .scene
                                    .entity(selected_entity.0.unwrap_or(Entity::DANGLING))
                                {
                                    if let Some(target) = resolve_aabb(e) {
                                        camera.focus_aabb(&target);
                                    } else if let Some(transform) = e.get::<&Transform>() {
                                        camera.focus(transform.translation, 10.0);
                                    }
                                }
                            }
                        }
                    } else if gui.egui.input_mut(|i| i.consume_shortcut(&SHORTCUT_GAZE)) {
                        let (d, pos) = renderer
                            .read()
                            .gbuffer
                            .depth_buffer_distance_pos_center(&camera);
                        if d.is_finite() {
                            camera.focus(pos, 10.0);
                        }
                    } else if gui
                        .egui
                        .input_mut(|i| i.consume_shortcut(&SHORTCUT_MAP_SWAP))
                    {
                        let mut maps = resources.get_mut::<MapList>().unwrap();

                        (maps.current_map, maps.previous_map) =
                            (maps.previous_map, maps.current_map);
                        maps.updated = maps.previous_map != maps.current_map;
                    }
                }
                last_frame = Instant::now();

                let window_dims = window.inner_size();

                unsafe {
                    renderer.read().clear_render_targets();

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

                    let mut maps = resources.get_mut::<MapList>().unwrap();

                    if let Some(map) = maps.current_map() {
                        {
                            let gb = gui_rendersettings.borrow();

                            let camera = resources.get_mut::<FpsCamera>().unwrap();
                            if let Some(driving_ent) = camera.driving {
                                if let Ok(mut transform) =
                                    map.scene.get::<&mut Transform>(driving_ent)
                                {
                                    transform.translation = camera.position;
                                    transform.rotation = camera.rotation;
                                }
                            }

                            for (e, (StaticInstances(instances, _), visible)) in map
                                .scene
                                .query::<(&StaticInstances, Option<&Visible>)>()
                                .iter()
                            {
                                if !visible.map_or(true, |v| v.0) {
                                    continue;
                                }

                                if instances.instance_count == 1
                                    && !camera.is_aabb_visible(&instances.occlusion_bounds[0])
                                {
                                    continue;
                                }

                                instances
                                    .draw(
                                        &renderer.read(),
                                        gb.renderlayer_statics,
                                        gb.renderlayer_statics_transparent,
                                        gb.renderlayer_statics_decals,
                                        e,
                                    )
                                    .unwrap();
                            }

                            if gb.renderlayer_terrain {
                                for (e, (terrain, visible)) in
                                    map.scene.query::<(&Terrain, Option<&Visible>)>().iter()
                                {
                                    if !visible.map_or(true, |v| v.0) {
                                        continue;
                                    }

                                    terrain.0.draw(&renderer.read(), e).unwrap();
                                }
                            }

                            for (e, (transform, rp, group, water, visible)) in map
                                .scene
                                .query::<(
                                    &Transform,
                                    &ResourcePoint,
                                    Option<&ActivityGroup>,
                                    Option<&Water>,
                                    Option<&Visible>,
                                )>()
                                .iter()
                            {
                                if !visible.map_or(true, |v| v.0) {
                                    continue;
                                }

                                if !gb.renderlayer_water && water.is_some() {
                                    continue;
                                }

                                if let (Some(group), Some(group_filters)) =
                                    (group, resources.get::<ActivityGroupFilter>())
                                {
                                    if !group_filters.filters.get(&group.0).unwrap_or(&true) {
                                        continue;
                                    }
                                }

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

                                if let Some(ent) = map.entity_renderers.get(&rp.entity_key()) {
                                    let mm = transform.to_mat4();

                                    // let mesh_to_world = Mat4::from_cols(
                                    //     mm.x_axis.truncate().extend(mm.w_axis.x),
                                    //     mm.y_axis.truncate().extend(mm.w_axis.y),
                                    //     mm.z_axis.truncate().extend(mm.w_axis.z),
                                    //     mm.w_axis,
                                    // );

                                    rp.entity_cbuffer.data().mesh_to_world = mm;

                                    if ent
                                        .draw(
                                            &renderer.read(),
                                            rp.entity_cbuffer.buffer().clone(),
                                            e,
                                        )
                                        .is_err()
                                    {
                                        renderer.write().push_fiddlesticks(*transform, Some(e));
                                    }
                                } else if rp.resource.is_entity() {
                                    // cohae: This will occur when there's no entitymodel for the given entity. Keeping it in just as a reminder of unimplemented entity rendering stuffs
                                    renderer.write().push_fiddlesticks(*transform, Some(e));
                                }
                            }

                            for (e, (transform, em)) in
                                map.scene.query::<(&Transform, &EntityModel)>().iter()
                            {
                                let mm = transform.to_mat4();

                                let mesh_to_world = Mat4::from_cols(
                                    mm.x_axis.truncate().extend(mm.w_axis.x),
                                    mm.y_axis.truncate().extend(mm.w_axis.y),
                                    mm.z_axis.truncate().extend(mm.w_axis.z),
                                    mm.w_axis,
                                );

                                em.1.data().mesh_to_world = mesh_to_world;

                                if em
                                    .0
                                    .draw(&renderer.read(), em.1.buffer().clone(), e)
                                    .is_err()
                                {
                                    renderer.write().push_fiddlesticks(*transform, Some(e));
                                }
                            }
                        }

                        // Find the smallest cubemap volume that the camera is in and set it as the current cubemap
                        let camera = resources.get::<FpsCamera>().unwrap();
                        let mut smallest_volume = f32::MAX;
                        let mut smallest_volume_entity = hecs::Entity::DANGLING;
                        for (e, (transform, volume)) in
                            map.scene.query::<(&Transform, &CubemapVolume)>().iter()
                        {
                            if volume.1.volume() < smallest_volume
                                && volume
                                    .1
                                    .contains_point_oriented(camera.position, transform.rotation)
                            {
                                smallest_volume = volume.1.volume();
                                smallest_volume_entity = e;
                            }
                        }

                        if let Ok(cubemap) = map.scene.get::<&CubemapVolume>(smallest_volume_entity)
                        {
                            if let Some(mut cr) = resources.get_mut::<CurrentCubemap>() {
                                cr.0 = Some(cubemap.2.clone());
                                cr.1 = Some(ExtendedHash::Hash32(cubemap.0));
                            }
                        } else if let Some(mut cr) = resources.get_mut::<CurrentCubemap>() {
                            cr.0 = None;
                        }

                        let mut debugshapes = resources.get_mut::<DebugShapes>().unwrap();
                        let selected = resources.get::<SelectedEntity>().unwrap();
                        for (e, (ruler, visible)) in
                            map.scene.query::<(&Ruler, Option<&Visible>)>().iter()
                        {
                            if !visible.map_or(true, |v| v.0) {
                                continue;
                            }
                            draw_ruler(&mut debugshapes, ruler, start_time, Some(e), &selected);
                        }
                        for (e, (transform, sphere, visible)) in map
                            .scene
                            .query::<(&Transform, &Sphere, Option<&Visible>)>()
                            .iter()
                        {
                            if !visible.map_or(true, |v| v.0) {
                                continue;
                            }
                            draw_sphere(
                                &mut debugshapes,
                                transform,
                                sphere,
                                start_time,
                                Some(e),
                                &selected,
                            );
                        }
                        for (e, (transform, beacon, visible)) in map
                            .scene
                            .query::<(&Transform, &Beacon, Option<&Visible>)>()
                            .iter()
                        {
                            if !visible.map_or(true, |v| v.0) {
                                continue;
                            }
                            draw_beacon(
                                &mut debugshapes,
                                transform,
                                beacon,
                                start_time,
                                Some(e),
                                &selected,
                            );
                        }
                        for (e, (route, visible)) in
                            map.scene.query::<(&Route, Option<&Visible>)>().iter()
                        {
                            if !visible.map_or(true, |v| v.0) {
                                continue;
                            }
                            draw_route(
                                &mut debugshapes,
                                route,
                                map.hash,
                                start_time,
                                Some(e),
                                &selected,
                            );
                        }
                    }

                    if let Some(map) = maps.current_map_mut() {
                        map.command_buffer.run_on(&mut map.scene);
                    }

                    drop(maps);

                    renderer.read().submit_frame(&resources);

                    gui.draw_frame(
                        window.clone(),
                        &mut resources,
                        |ctx, resources| {
                            update_channel_gui.open = config::with(|c| c.update_channel.is_none());
                            update_channel_gui.show(ctx, resources);
                            if update_channel_gui.open {
                                return PreDrawResult::Stop;
                            }

                            {
                                let mut loads = resources.get_mut::<LoadIndicators>().unwrap();
                                let mut update_check = resources.get_mut::<UpdateCheck>().unwrap();
                                {
                                    let check_running = update_check
                                        .0
                                        .as_ref()
                                        .map_or(false, |v| v.poll().is_pending());

                                    let indicator =
                                        loads.entry("update_check".to_string()).or_insert_with(
                                            || LoadIndicator::new("Checking for updates"),
                                        );

                                    if indicator.active != check_running {
                                        indicator.restart();
                                    }

                                    indicator.active = check_running;
                                }

                                if update_check
                                    .0
                                    .as_ref()
                                    .map_or(false, |v| v.poll().is_ready())
                                {
                                    let update = update_check.0.take().unwrap().block_and_take();
                                    if let Some(update) = update {
                                        updater_gui = Some(UpdateDownload::new(update));
                                    }
                                }
                            }

                            if let Some(updater_gui_) = updater_gui.as_mut() {
                                if !updater_gui_.show(ctx, resources) {
                                    updater_gui = None;
                                }

                                return PreDrawResult::Stop;
                            }

                            PreDrawResult::Continue
                        },
                        |ctx, resources| {
                            activity_browser.show(ctx, resources);

                            let maplist = resources.get::<MapList>().unwrap();
                            if let Some(map) = maplist.current_map() {
                                if map.load_state == MapLoadState::Loading {
                                    egui::Window::new("Loading...")
                                        .title_bar(false)
                                        .resizable(false)
                                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                                        .show(ctx, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.spinner();
                                                ui.heading(format!("Loading map '{}'", map.name));
                                            })
                                        });
                                }
                            }
                        },
                    );

                    // TODO(cohae): This triggers when dragging as well, which is super annoying. Don't know if we can fix this without a proper egui response object though.
                    if gui.egui.input(|i| i.pointer.secondary_clicked())
                        && !gui.egui.wants_pointer_input()
                        && !resources.get::<SelectedEntity>().unwrap().1
                    {
                        if let Some(mouse_pos) = gui.egui.pointer_interact_pos() {
                            let id = renderer.read().gbuffer.pick_buffer_read(
                                (mouse_pos.x as f64 * window.scale_factor()).round() as usize,
                                (mouse_pos.y as f64 * window.scale_factor()).round() as usize,
                            );
                            let maps = resources.get::<MapList>().unwrap();

                            if let Some(map) = maps.current_map() {
                                if id != u32::MAX {
                                    *resources.get_mut::<SelectedEntity>().unwrap() =
                                        SelectedEntity(
                                            Some(map.scene.find_entity_from_id(id)),
                                            true,
                                            Instant::now(),
                                        );
                                } else {
                                    *resources.get_mut::<SelectedEntity>().unwrap() =
                                        SelectedEntity(None, true, Instant::now());
                                }
                            }
                        }
                    }

                    hotkeys::process_hotkeys(&gui.egui, &mut resources);

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

                    let gdb = gui_debug.borrow();
                    let mut resource_filters: HashMap<String, bool> = Default::default();
                    for (i, enabled) in gdb.map_resource_filter.iter().enumerate() {
                        resource_filters.insert(MapResource::index_to_id(i).to_string(), *enabled);
                    }

                    c.resources.show_resources = gdb.show_map_resources;
                    c.resources.map_resource_label_background = gdb.map_resource_label_background;
                    c.resources.resource_distance_limit = gdb.map_resource_distance_limit_enabled;
                    c.resources.filters = resource_filters;

                    let render_settings = resources.get_mut::<RenderSettings>().unwrap();
                    c.render_settings.draw_crosshair = render_settings.draw_crosshair;
                });
                config::persist();
            }
            _ => (),
        }
    });
}

fn get_rainbow_color(start_time: Instant) -> [u8; 3] {
    Hsva {
        h: (start_time.elapsed().as_secs_f32() * 0.30) % 1.0,
        s: 1.0,
        v: 1.0,
        a: 1.0,
    }
    .to_srgb()
}

fn get_selected_color<const N: usize>(
    selected: &SelectedEntity,
    e: Option<Entity>,
    c: [u8; N],
) -> [u8; N] {
    let select_color = [255, 153, 51, 255];
    let elapsed = ease_out_exponential((selected.2.elapsed().as_secs_f32() / 1.4).min(1.0));
    if selected.0 == e && elapsed < 1.0 {
        let mut ret = [0; N];
        for i in 0..N.min(4) {
            ret[i] =
                (select_color[i] as f32 * (1.0 - elapsed) + (c[i] as f32 * elapsed)).round() as u8;
        }
        ret
    } else {
        c
    }
}

fn draw_ruler(
    debugshapes: &mut DebugShapes,
    ruler: &Ruler,
    start_time: Instant,
    entity: Option<Entity>,
    selected: &SelectedEntity,
) {
    let color = if ruler.rainbow {
        get_selected_color::<3>(selected, entity, get_rainbow_color(start_time))
    } else {
        get_selected_color::<3>(selected, entity, ruler.color)
    };

    debugshapes.cross(ruler.start, ruler.scale, color);
    debugshapes.cross(ruler.end, ruler.scale, color);
    debugshapes.line_dotted(ruler.start, ruler.end, color, ruler.scale, 0.5, 0.5);

    let ruler_center = (ruler.start + ruler.end) / 2.0;
    debugshapes.text(
        prettify_distance(ruler.length()),
        ruler_center,
        egui::Align2::CENTER_BOTTOM,
        [255, 255, 255],
    );

    if ruler.show_individual_axis {
        let end_x = Vec3::new(ruler.end.x, ruler.start.y, ruler.start.z);
        let end_y = Vec3::new(ruler.start.x, ruler.end.y, ruler.start.z);
        let end_z = Vec3::new(ruler.start.x, ruler.start.y, ruler.end.z);

        debugshapes.line(ruler.start, end_x, color);
        debugshapes.line(ruler.start, end_y, color);
        debugshapes.line(ruler.start, end_z, color);

        let length_x = (ruler.start - end_x).length();
        let length_y = (ruler.start - end_y).length();
        let length_z = (ruler.start - end_z).length();

        let center_x = (ruler.start + end_x) / 2.0;
        let center_y = (ruler.start + end_y) / 2.0;
        let center_z = (ruler.start + end_z) / 2.0;

        debugshapes.text(
            format!("X: {}", prettify_distance(length_x)),
            center_x,
            egui::Align2::LEFT_CENTER,
            [255, 255, 255],
        );

        debugshapes.text(
            format!("Y: {}", prettify_distance(length_y)),
            center_y,
            egui::Align2::RIGHT_CENTER,
            [255, 255, 255],
        );

        debugshapes.text(
            format!("Z: {}", prettify_distance(length_z)),
            center_z,
            egui::Align2::RIGHT_CENTER,
            [255, 255, 255],
        );
    }

    if ruler.marker_interval > 0.0 {
        let sphere_color = keep_color_bright(invert_color(color));
        let sphere_color = [sphere_color[0], sphere_color[1], sphere_color[2], 192];

        let mut current = 0.0;
        while current < ruler.length() {
            if current > 0.0 {
                let pos = ruler.start + ruler.direction() * current;

                debugshapes.sphere(
                    pos,
                    ruler.scale * 0.20,
                    sphere_color,
                    DebugDrawFlags::DRAW_NORMAL,
                    None,
                );
            }

            current += ruler.marker_interval;
        }
    }
    debugshapes.cube_extents(
        (ruler.start + ruler.end) / 2.0,
        Vec3::new(ruler.length() / 2.0, ruler.scale / 2.0, ruler.scale / 2.0),
        Quat::from_rotation_arc(Vec3::X, (ruler.end - ruler.start).normalize()),
        color,
        true,
        DebugDrawFlags::DRAW_PICK,
        entity,
    )
}

fn draw_sphere_skeleton<C: Into<Color> + Copy>(
    debugshapes: &mut DebugShapes,
    pos: Vec3,
    radius: f32,
    detail: u8,
    color: C,
) {
    for t in 0..detail {
        debugshapes.circle(
            pos,
            Vec3::new(
                radius * (t as f32 * PI / detail as f32).sin(),
                radius * (t as f32 * PI / detail as f32).cos(),
                0.0,
            ),
            4 * detail,
            color,
        );
    }
    debugshapes.circle(pos, Vec3::new(0.0, 0.0, radius), 4 * detail, color);
}

fn draw_sphere(
    debugshapes: &mut DebugShapes,
    transform: &Transform,
    sphere: &Sphere,
    start_time: Instant,
    entity: Option<Entity>,
    selected: &SelectedEntity,
) {
    let color = if sphere.rainbow {
        let c = get_rainbow_color(start_time);
        get_selected_color::<4>(selected, entity, [c[0], c[1], c[2], sphere.color[3]])
    } else {
        get_selected_color::<4>(selected, entity, sphere.color)
    };

    let color_opaque = [color[0], color[1], color[2]];
    let cross_color = keep_color_bright(invert_color(color_opaque));
    debugshapes.cross(
        transform.translation,
        0.25 * transform.radius(),
        cross_color,
    );

    draw_sphere_skeleton(
        debugshapes,
        transform.translation,
        transform.radius(),
        sphere.detail,
        color,
    );

    debugshapes.text(
        prettify_distance(transform.radius()),
        transform.translation,
        egui::Align2::CENTER_BOTTOM,
        [255, 255, 255],
    );
    debugshapes.sphere(
        transform.translation,
        transform.radius(),
        color,
        DebugDrawFlags::DRAW_NORMAL | DebugDrawFlags::DRAW_PICK,
        entity,
    );
}

fn draw_beacon(
    debugshapes: &mut DebugShapes,
    transform: &Transform,
    beacon: &Beacon,
    start_time: Instant,
    entity: Option<Entity>,
    selected: &SelectedEntity,
) {
    const BEAM_HEIGHT: f32 = 5000.0;
    const BASE_RADIUS: f32 = 0.1;
    let color: [u8; 4] = get_selected_color::<4>(
        selected,
        entity,
        [
            beacon.color[0],
            beacon.color[1],
            beacon.color[2],
            (150.0 + (start_time.elapsed().as_secs_f32() * 2.0 * PI * beacon.freq).sin() * 50.0)
                as u8,
        ],
    );
    debugshapes.sphere(
        transform.translation,
        BASE_RADIUS,
        color,
        DebugDrawFlags::DRAW_NORMAL,
        None,
    );
    debugshapes.line(
        transform.translation + Vec3::Z * BASE_RADIUS,
        transform.translation + Vec3::Z * BEAM_HEIGHT,
        color,
    );
    debugshapes.cube_extents(
        transform.translation + Vec3::Z * BEAM_HEIGHT / 2.0,
        Vec3::new(BASE_RADIUS, BASE_RADIUS, BEAM_HEIGHT / 2.0),
        Quat::IDENTITY,
        color,
        true,
        DebugDrawFlags::DRAW_PICK,
        entity,
    );
}

fn draw_route(
    debugshapes: &mut DebugShapes,
    route: &Route,
    current_hash: TagHash,
    start_time: Instant,
    entity: Option<Entity>,
    selected: &SelectedEntity,
) {
    let color = if route.rainbow {
        get_selected_color::<3>(selected, entity, get_rainbow_color(start_time))
    } else {
        get_selected_color::<3>(selected, entity, route.color)
    };

    const BASE_RADIUS: f32 = 0.1;
    let mut prev_is_local = false;
    for i in 0..route.path.len() {
        if let Some(node) = route.path.get(i) {
            let node_is_local = node.map_hash.map_or(true, |h| h == current_hash);
            let next_node = route.path.get(i + 1);
            let next_is_local = next_node.map_or(false, |node| {
                node.map_hash.map_or(false, |h| h == current_hash)
            });

            if !node_is_local {
                if prev_is_local || next_is_local || route.show_all {
                    draw_sphere_skeleton(debugshapes, node.pos, BASE_RADIUS, 2, color);
                }
            } else {
                debugshapes.sphere(
                    node.pos,
                    BASE_RADIUS,
                    color,
                    DebugDrawFlags::DRAW_NORMAL,
                    None,
                );
            }

            if node_is_local || prev_is_local || next_is_local {
                if let Some(label) = node.label.as_ref() {
                    debugshapes.text(
                        label.to_string(),
                        node.pos + route.scale / 2.0 * Vec3::Z,
                        egui::Align2::CENTER_BOTTOM,
                        [255, 255, 255],
                    );
                }
            }
            prev_is_local = node_is_local;

            if next_node.is_some() {
                let next_node = next_node.unwrap();
                let segment_length = (next_node.pos - node.pos).length();

                if !(route.show_all || node_is_local || next_is_local) {
                    continue;
                }

                debugshapes.line_dotted(
                    next_node.pos,
                    node.pos,
                    color,
                    route.scale,
                    if next_node.is_teleport { 0.10 } else { 0.75 },
                    if next_node.is_teleport { 1.5 } else { 0.5 },
                );
                if route.marker_interval > 0.0 {
                    let sphere_color = keep_color_bright(invert_color(color));
                    let sphere_color = [sphere_color[0], sphere_color[1], sphere_color[2], 192];

                    let mut current = 0.0;
                    while current < segment_length {
                        if current > 0.0 {
                            let pos = node.pos + (next_node.pos - node.pos).normalize() * current;

                            debugshapes.sphere(
                                pos,
                                route.scale * 0.20,
                                sphere_color,
                                DebugDrawFlags::DRAW_NORMAL,
                                None,
                            );
                        }

                        current += route.marker_interval;
                    }
                }
                debugshapes.cube_extents(
                    (node.pos + next_node.pos) / 2.0,
                    Vec3::new(segment_length / 2.0, route.scale / 2.0, route.scale / 2.0),
                    Quat::from_rotation_arc(Vec3::X, (next_node.pos - node.pos).normalize()),
                    color,
                    true,
                    DebugDrawFlags::DRAW_PICK,
                    entity,
                )
            } else {
                debugshapes.cube_extents(
                    node.pos,
                    Vec3::new(route.scale / 2.0, route.scale / 2.0, route.scale / 2.0),
                    Quat::IDENTITY,
                    color,
                    true,
                    DebugDrawFlags::DRAW_PICK,
                    entity,
                )
            }
        }
    }
}

fn load_render_globals(renderer: &Renderer) {
    let globals = package_manager()
        .read_named_tag_struct::<SRenderGlobals>("render_globals")
        .expect("Failed to read render globals");

    //     println!("{globals:#?}");
    //     for (i, s) in globals.unk8[0].unk8.scopes.iter().enumerate() {
    //         println!(
    //             "scope #{i}: {} cb{}",
    //             *s.name,
    //             s.scope
    //                 .iter_stages()
    //                 .find(|v| v.constant_buffer.is_some())
    //                 .map_or(u32::MAX, |v| v.constant_buffer_slot)
    //         );
    //         // println!("scope #{i}: {} ({})", *s.name, s.scope.hash());
    //         // if s.scope.stage_vertex.constant_buffer.is_some() {
    //         //     println!(
    //         //         "---- VS cb{} ({} bytes)",
    //         //         s.scope.stage_vertex.constant_buffer_slot,
    //         //         buffer_size(s.scope.stage_vertex.constant_buffer)
    //         //     );
    //         //     decompile_tfx(&s.scope.stage_vertex);
    //         // }
    //         // if s.scope.stage_pixel.constant_buffer.is_some() {
    //         //     println!(
    //         //         "---- PS cb{} ({} bytes)",
    //         //         s.scope.stage_pixel.constant_buffer_slot,
    //         //         buffer_size(s.scope.stage_pixel.constant_buffer)
    //         //     );
    //         //     decompile_tfx(&s.scope.stage_pixel);
    //         // }
    //         // if s.scope.stage_geometry.constant_buffer.is_some() {
    //         //     println!(
    //         //         "---- GS cb{} ({} bytes)",
    //         //         s.scope.stage_geometry.constant_buffer_slot,
    //         //         buffer_size(s.scope.stage_geometry.constant_buffer)
    //         //     );
    //         //     decompile_tfx(&s.scope.stage_geometry);
    //         // }
    //         // if s.scope.stage_hull.constant_buffer.is_some() {
    //         //     println!(
    //         //         "---- HS cb{} ({} bytes)",
    //         //         s.scope.stage_hull.constant_buffer_slot,
    //         //         buffer_size(s.scope.stage_hull.constant_buffer)
    //         //     );
    //         //     decompile_tfx(&s.scope.stage_hull);
    //         // }
    //         // if s.scope.stage_compute.constant_buffer.is_some() {
    //         //     println!(
    //         //         "---- CS cb{} ({} bytes)",
    //         //         s.scope.stage_compute.constant_buffer_slot,
    //         //         buffer_size(s.scope.stage_compute.constant_buffer)
    //         //     );
    //         //     decompile_tfx(&s.scope.stage_compute);
    //         // }
    //         // if s.scope.stage_domain.constant_buffer.is_some() {
    //         //     println!(
    //         //         "---- DS cb{} ({} bytes)",
    //         //         s.scope.stage_domain.constant_buffer_slot,
    //         //         buffer_size(s.scope.stage_domain.constant_buffer)
    //         //     );
    //         //     decompile_tfx(&s.scope.stage_domain);
    //         // }
    // }

    let mut techniques: HashMap<String, TagHash> = HashMap::default();
    for t in &globals.unk8[0].unk8.unk20 {
        // println!("technique #{i}: {}, {}", *t.name, t.technique);
        techniques.insert(t.name.to_string(), t.technique);
    }

    let technique_tag = techniques["deferred_shading_no_atm"];
    let technique = Technique::load(
        renderer,
        package_manager().read_tag_struct(technique_tag).unwrap(),
        technique_tag,
        true,
    );

    load_shaders(renderer, &technique);

    renderer
        .render_data
        .data_mut()
        .technique_deferred_shading_no_atm = Some(technique);

    renderer.render_data.data_mut().iridescence_lookup = {
        let texture_tag = globals.unk8[0].unk8.unk30.iridescence_lookup_texture;
        Texture::load(&renderer.dcs, ExtendedHash::Hash32(texture_tag)).ok()
    };

    info!("Loaded deferred_shading_no_atm");
}

// fn buffer_size(tag: TagHash) -> usize {
//     let eeee = package_manager().get_entry(tag).unwrap().reference;
//     package_manager().read_tag(TagHash(eeee)).unwrap().len()
// }

// fn decompile_tfx(s: &SScopeStage) {
//     if let Ok(opcodes) = TfxBytecodeOp::parse_all(&s.bytecode, binrw::Endian::Little) {
//         match TfxBytecodeDecompiler::decompile(opcodes, &s.bytecode_constants) {
//             Ok(o) => println!("{}", o.pretty_print()),
//             Err(e) => error!("Failed to decompile bytecode: {}", e),
//         }
//     }
// }

// fn buffer_size(tag: TagHash) -> usize {
//     let eeee = package_manager().get_entry(tag).unwrap().reference;
//     package_manager().read_tag(TagHash(eeee)).unwrap().len()
// }

fn load_shaders(renderer: &Renderer, m: &Technique) {
    let mut render_data = renderer.render_data.data_mut();

    if let Some(v) = package_manager().get_entry(m.stage_vertex.shader.shader) {
        let _span = debug_span!("load vshader", shader = ?m.stage_vertex.shader).entered();

        let vs_data = package_manager().read_tag(v.reference).unwrap();

        let mut vs_cur = Cursor::new(&vs_data);
        let dxbc_header: DxbcHeader = vs_cur.read_le().unwrap();
        let input_sig = get_input_signature(&mut vs_cur, &dxbc_header).unwrap();

        let layout_converted = input_sig
            .elements
            .iter()
            .map(|e| InputElement::from_dxbc(e, e.component_type == DxbcInputType::Float, false))
            .collect_vec();

        let shader = unsafe {
            let v = renderer
                .dcs
                .device
                .CreateVertexShader(&vs_data, None)
                .context("Failed to load vertex shader")
                .unwrap();

            (v, layout_converted, vs_data)
        };

        render_data
            .vshaders
            .insert(m.stage_vertex.shader.shader, shader);
    }

    // return Ok(());

    if let Some(v) = package_manager().get_entry(m.stage_pixel.shader.shader) {
        let _span = debug_span!("load pshader", shader = ?m.stage_pixel.shader.shader).entered();

        let ps_data = package_manager().read_tag(v.reference).unwrap();

        let mut ps_cur = Cursor::new(&ps_data);
        let dxbc_header: DxbcHeader = ps_cur.read_le().unwrap();
        let output_sig = get_output_signature(&mut ps_cur, &dxbc_header).unwrap();

        let layout_converted = output_sig
            .elements
            .iter()
            .map(|e| InputElement::from_dxbc(e, e.component_type == DxbcInputType::Float, false))
            .collect_vec();

        let shader = unsafe {
            let v = renderer
                .dcs
                .device
                .CreatePixelShader(&ps_data, None)
                .context("Failed to load pixel shader")
                .unwrap();

            (v, layout_converted)
        };

        render_data
            .pshaders
            .insert(m.stage_pixel.shader.shader, shader);
    }
}
