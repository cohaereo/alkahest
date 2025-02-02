use std::sync::{atomic::Ordering, Arc};

use alkahest_data::text::{StringContainer, StringContainerShared};
use alkahest_renderer::{
    camera::{Camera, Viewport},
    ecs::{
        channels::object_channels_discovery_system,
        new_scene,
        resources::SelectedEntity,
        tags::{NodeFilter, NodeFilterSet},
        Scene,
    },
    gpu::{texture::LOW_RES, GpuContext},
    gpu_event, gpu_profile_event,
    input::InputState,
    renderer::{Renderer, RendererShared},
};
use bevy_ecs::system::RunSystemOnce;
use bevy_tasks::{ComputeTaskPool, TaskPool};
use egui::{Key, KeyboardShortcut, Modifiers};
use gilrs::{EventType, Gilrs};
use glam::Vec2;
use strum::IntoEnumIterator;
use transform_gizmo_egui::{EnumSet, Gizmo, GizmoConfig, GizmoOrientation};
use windows::core::HRESULT;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{MouseScrollDelta, WindowEvent},
    event_loop::EventLoop,
    platform::run_on_demand::EventLoopExtRunOnDemand,
};

use crate::{
    config,
    gui::{
        activity_select::{get_map_name, set_activity, ActivityBrowser, CurrentActivity},
        console,
        context::{GuiContext, GuiViewManager, HiddenWindows},
        gizmo::draw_transform_gizmos,
        hotkeys,
        inspector::FnvWordlist,
        updater::{ChannelSelector, UpdateDownload},
        SelectionGizmoMode,
    },
    maplist::{Map, MapList},
    resources::AppResources,
    updater::UpdateCheck,
    util::{
        action::{ActionBuffer, ActionList},
        iron,
    },
    ApplicationArgs,
};

pub struct AlkahestApp {
    pub window: Arc<winit::window::Window>,
    pub event_loop: EventLoop<()>,

    pub gctx: Arc<GpuContext>,
    pub gui: GuiContext,
    pub resources: AppResources,

    gilrs: Gilrs,
    last_cursor_pos: Option<PhysicalPosition<f64>>,

    renderer: RendererShared,
    scratch_map: Scene,

    update_channel_gui: ChannelSelector,
    updater_gui: Option<UpdateDownload>,

    next_config_save: std::time::Instant,
}

impl AlkahestApp {
    const CONFIG_SAVE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5 * 60);

    pub fn new(
        event_loop: EventLoop<()>,
        icon: &winit::window::Icon,
        args: ApplicationArgs,
    ) -> Self {
        iron::set_policy(iron::Policy::Disabled);
        alkahest_renderer::gpu::DESKTOP_DISPLAY_MODE
            .store(iron::get_content_policy(), Ordering::SeqCst);

        let window = winit::window::WindowBuilder::new()
            .with_title("Alkahest")
            .with_min_inner_size(PhysicalSize::new(640, 360))
            .with_inner_size(config::with(|c| {
                PhysicalSize::new(c.window.width, c.window.height)
            }))
            .with_position(config::with(|c| {
                PhysicalPosition::new(c.window.pos_x, c.window.pos_y)
            }))
            .with_maximized(config!().window.maximised)
            .with_fullscreen(if config!().window.fullscreen {
                Some(winit::window::Fullscreen::Borderless(None))
            } else {
                None
            })
            .with_window_icon(Some(icon.clone()))
            .build(&event_loop)
            .unwrap();
        let window = Arc::new(window);

        puffin::set_scopes_on(cfg!(feature = "profiler"));

        let gctx = Arc::new(GpuContext::create(&window).unwrap());
        let gui = GuiContext::create(&window, gctx.clone());
        let mut resources = AppResources::default();
        resources.insert(GuiViewManager::with_default_views());
        resources.insert(InputState::default());
        resources.insert(CurrentActivity(args.activity));
        resources.insert(SelectedEntity::default());
        resources.insert(args);
        resources.insert(window.clone());
        resources.insert(FnvWordlist::new());

        let mut maps = MapList::default();
        maps.maps.push(Map::create_empty("Empty Map"));
        resources.insert(maps);
        resources.insert(SelectionGizmoMode::default());
        resources.insert(HiddenWindows::default());
        resources.insert(ActionList::default());
        resources.insert(ActionBuffer::default());
        let renderer = Renderer::create(
            gctx.clone(),
            (window.inner_size().width, window.inner_size().height),
            false,
        )
        .unwrap();
        renderer.set_render_settings(config::with(|c| c.renderer.clone()));
        resources.insert(renderer.clone());
        let stringmap = Arc::new(StringContainer::load_all_global());
        resources.insert(stringmap);

        let gizmo = Gizmo::new(GizmoConfig {
            modes: EnumSet::all(),
            orientation: GizmoOrientation::Local,
            ..Default::default()
        });
        resources.insert(gizmo);

        resources
            .get_mut::<GuiViewManager>()
            .insert(ActivityBrowser::new(
                &resources.get::<StringContainerShared>(),
            ));

        resources.insert(UpdateCheck::default());
        let update_channel_gui = ChannelSelector {
            open: config::with(|c| c.update_channel.is_none()),
        };

        let updater_gui: Option<UpdateDownload> = None;

        if let Some(update_channel) = config::with(|c| c.update_channel) {
            resources.get_mut::<UpdateCheck>().start(update_channel);
        }

        let camera = Camera::new_fps(Viewport {
            size: glam::UVec2::new(1920, 1080),
            origin: glam::UVec2::new(0, 0),
        });
        resources.insert(camera);
        if let Some(acthash) = resources.get::<ApplicationArgs>().activity {
            set_activity(&resources, acthash).ok();
        } else if let Some(maphash) = resources.get::<ApplicationArgs>().map {
            let map_name = get_map_name(maphash, &resources.get::<StringContainerShared>())
                .unwrap_or_else(|_| format!("Unknown map {maphash}"));

            resources
                .get_mut::<MapList>()
                .set_maps(&resources, &[(maphash, map_name)]);
        }

        let mut node_filter_set = NodeFilterSet::default();
        config::with(|c| {
            for nf in NodeFilter::iter() {
                if c.visual.node_filters.contains(&nf.to_string()) {
                    node_filter_set.insert(nf);
                }
            }
        });
        resources.insert(node_filter_set);

        {
            let args = resources.get::<ApplicationArgs>();
            LOW_RES.store(args.low_res, std::sync::atomic::Ordering::Relaxed);
        }

        ComputeTaskPool::get_or_init(TaskPool::default);

        Self {
            window,
            event_loop,
            gctx,
            gui,
            resources,
            gilrs: Gilrs::new().unwrap(),
            last_cursor_pos: None,
            renderer,
            scratch_map: new_scene(),
            update_channel_gui,
            updater_gui,
            next_config_save: std::time::Instant::now() + Self::CONFIG_SAVE_INTERVAL,
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let AlkahestApp {
            window,
            event_loop,
            gui,
            gctx,
            resources,
            last_cursor_pos,
            renderer,
            scratch_map,
            update_channel_gui,
            updater_gui,
            gilrs,
            next_config_save,
            ..
        } = self;

        let mut active_gamepad = None;

        event_loop.run_on_demand(move |event, target| {
            if let winit::event::Event::WindowEvent { event, .. } = event {
                let egui_event_response = gui.handle_event(window, &event);
                if !egui_event_response.consumed {
                    resources.get_mut::<InputState>().handle_event(&event);
                }

                match event {
                    WindowEvent::CloseRequested => {
                        target.exit();
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let input = resources.get::<InputState>();
                        if let Some(ref mut p) = last_cursor_pos {
                            let delta = (position.x - p.x, position.y - p.y);
                            if (input.mouse_right() || input.mouse_middle())
                                && !egui_event_response.consumed
                            {
                                resources
                                    .get_mut::<Camera>()
                                    .update_mouse((delta.0 as f32, delta.1 as f32).into(), 0.0);

                                if delta != (0.0, 0.0) {
                                    window.set_cursor_position(*p).ok();
                                }

                                window.set_cursor_visible(false);
                            } else {
                                window.set_cursor_visible(true);
                                *last_cursor_pos = Some(position);
                            }
                        } else {
                            window.set_cursor_visible(true);
                            *last_cursor_pos = Some(position);
                        }
                    }
                    WindowEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(_scroll_x, scroll_y),
                        ..
                    } => {
                        if !egui_event_response.consumed {
                            resources
                                .get_mut::<Camera>()
                                .update_mouse(Vec2::ZERO, scroll_y);
                        }
                    }
                    WindowEvent::Resized(new_dims) => {
                        if let Some(swap_chain) = gctx.swap_chain.as_ref() {
                            let _ = gui.renderer.as_mut().map(|renderer| {
                                let _ = renderer
                                    .resize_buffers(swap_chain, || {
                                        gctx.resize_swapchain(new_dims.width, new_dims.height);
                                        HRESULT(0)
                                    })
                                    .unwrap();
                            });
                        }

                        renderer.resize_buffers(new_dims.width, new_dims.height);

                        resources.get_mut::<Camera>().set_viewport(Viewport {
                            size: glam::UVec2::new(new_dims.width, new_dims.height),
                            origin: glam::UVec2::ZERO,
                        });

                        config::with_mut(|c| {
                            (c.window.width, c.window.height) = (new_dims.width, new_dims.height)
                        });
                    }
                    WindowEvent::RedrawRequested => {
                        if *next_config_save < std::time::Instant::now() {
                            config::try_persist().ok();
                            *next_config_save =
                                std::time::Instant::now() + Self::CONFIG_SAVE_INTERVAL;
                        }

                        resources.get_mut::<SelectedEntity>().changed_this_frame = false;
                        renderer.data.lock().asset_manager.poll();

                        gctx.begin_frame();

                        {
                            gpu_profile_event!(gctx, "main");
                            if gui.input_mut(|i| {
                                i.consume_shortcut(&KeyboardShortcut::new(
                                    Modifiers::ALT,
                                    Key::Enter,
                                ))
                            }) {
                                if window.fullscreen().is_some() {
                                    window.set_fullscreen(None);
                                } else {
                                    window.set_fullscreen(Some(
                                        winit::window::Fullscreen::Borderless(
                                            window.current_monitor(),
                                        ),
                                    ));
                                }

                                config::with_mut(|c| {
                                    c.window.fullscreen = window.fullscreen().is_some();
                                });
                            }

                            {
                                let mut action_list = resources.get_mut::<ActionList>();
                                action_list.process(resources);
                            }

                            resources
                                .get_mut::<Camera>()
                                .update(&resources.get::<InputState>(), renderer.delta_time as f32);

                            // Process gamepad input
                            {
                                // Examine new events
                                while let Some(gilrs::Event { id, event, .. }) = gilrs.next_event()
                                {
                                    active_gamepad = Some(id);

                                    if let EventType::ButtonPressed {
                                        0: gilrs::Button::Start,
                                        ..
                                    } = event
                                    {
                                        let mut gui_views = resources.get_mut::<GuiViewManager>();
                                        gui_views.hide_views = !gui_views.hide_views;
                                    }
                                }

                                let mut camera = resources.get_mut::<Camera>();
                                // You can also use cached gamepad state
                                if let Some(gamepad) = active_gamepad.map(|id| gilrs.gamepad(id)) {
                                    let left_x = gamepad
                                        .axis_data(gilrs::Axis::LeftStickX)
                                        .map(|v| v.value())
                                        .unwrap_or_default();
                                    let left_y = gamepad
                                        .axis_data(gilrs::Axis::LeftStickY)
                                        .map(|v| v.value())
                                        .unwrap_or_default();
                                    let right_x = gamepad
                                        .axis_data(gilrs::Axis::RightStickX)
                                        .map(|v| v.value())
                                        .unwrap_or_default();
                                    let right_y = gamepad
                                        .axis_data(gilrs::Axis::RightStickY)
                                        .map(|v| v.value())
                                        .unwrap_or_default();

                                    camera.update_gamepad(
                                        (left_x, left_y).into(),
                                        (right_x, right_y).into(),
                                        1.0 + if gamepad.is_pressed(gilrs::Button::LeftTrigger2) {
                                            3.0
                                        } else {
                                            0.0
                                        } + if gamepad.is_pressed(gilrs::Button::RightTrigger2) {
                                            10.0
                                        } else {
                                            0.0
                                        },
                                        renderer.delta_time as f32,
                                    );
                                }
                            }

                            let mut maps = resources.get_mut::<MapList>();
                            maps.update_maps(resources);

                            if let Some(map) = maps.current_map_mut() {
                                map.scene.run_system_once_with(
                                    resources.get::<RendererShared>().clone(),
                                    object_channels_discovery_system,
                                );

                                map.update();
                            }

                            let scene = maps
                                .current_map_mut()
                                .map(|m| &mut m.scene)
                                .unwrap_or(scratch_map);

                            renderer.render_world(&*resources.get::<Camera>(), scene, resources);
                        }

                        unsafe {
                            renderer.gpu.lock_context().OMSetRenderTargets(
                                Some(&[renderer.gpu.swapchain_target.read().clone()]),
                                None,
                            );
                        }

                        renderer
                            .gpu
                            .begin_event_span("interface_and_hud", "")
                            .scoped(|| {
                                gpu_profile_event!(renderer.gpu, "egui");
                                gui.draw_frame(window, |ctx, ectx| {
                                    update_channel_gui.open =
                                        config::with(|c| c.update_channel.is_none());
                                    update_channel_gui.show(ectx, resources);
                                    if update_channel_gui.open {
                                        return;
                                    }

                                    {
                                        // let mut loads = resources.get_mut::<LoadIndicators>().unwrap();
                                        let mut update_check = resources.get_mut::<UpdateCheck>();
                                        // {
                                        //     let check_running = update_check
                                        //         .0
                                        //         .as_ref()
                                        //         .map_or(false, |v| v.poll().is_pending());
                                        //
                                        //     let indicator =
                                        //         loads.entry("update_check".to_string()).or_insert_with(
                                        //             || LoadIndicator::new("Checking for updates"),
                                        //         );
                                        //
                                        //     if indicator.active != check_running {
                                        //         indicator.restart();
                                        //     }
                                        //
                                        //     indicator.active = check_running;
                                        // }

                                        if update_check
                                            .0
                                            .as_ref()
                                            .map_or(false, |v| v.poll().is_ready())
                                        {
                                            let update =
                                                update_check.0.take().unwrap().block_and_take();
                                            if let Some(update) = update {
                                                *updater_gui = Some(UpdateDownload::new(update));
                                            }
                                        }
                                    }

                                    if let Some(updater_gui_) = updater_gui.as_mut() {
                                        if !updater_gui_.show(ectx, resources) {
                                            *updater_gui = None;
                                        }

                                        return;
                                    }

                                    let mut gui_views = resources.get_mut::<GuiViewManager>();
                                    gui_views.draw(ectx, window, resources, ctx);

                                    if !gui_views.hide_views {
                                        draw_transform_gizmos(renderer, ectx, resources);
                                    }

                                    drop(gui_views);
                                    hotkeys::process_hotkeys(ectx, resources);
                                });
                            });

                        window.pre_present_notify();
                        gctx.present(config::with(|c| c.renderer.vsync));

                        window.request_redraw();
                        profiling::finish_frame!();

                        // Slow the app to 10fps when it's window is out of focus
                        if !window.has_focus() {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }

                        console::process_queued_commands(resources);
                        if let Some(picked_id) = renderer.pickbuffer.finish_request() {
                            let mut selected = resources.get_mut::<SelectedEntity>();
                            if !selected.changed_this_frame {
                                if picked_id != u32::MAX {
                                    let maps = resources.get::<MapList>();
                                    if let Some(map) = maps.current_map() {
                                        selected.select_option(
                                            map.scene
                                                .iter_entities()
                                                .find(|er| er.id().index() == picked_id)
                                                .map(|er| er.id()),
                                        );
                                    }
                                } else {
                                    selected.deselect();
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        })?;

        Ok(())
    }
}

impl Drop for AlkahestApp {
    fn drop(&mut self) {
        config::persist();
    }
}
