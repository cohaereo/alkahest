use std::sync::Arc;

use alkahest_data::text::{GlobalStringmap, StringMapShared};
use alkahest_renderer::{
    camera::{Camera, Viewport},
    ecs::{
        resources::SelectedEntity,
        tags::{NodeFilter, NodeFilterSet},
        Scene,
    },
    gpu::GpuContext,
    gpu_event,
    input::InputState,
    renderer::{Renderer, RendererSettings, RendererShared},
};
use egui::{Key, KeyboardShortcut, Modifiers};
use glam::Vec2;
use strum::IntoEnumIterator;
use transform_gizmo_egui::{enum_set, Gizmo, GizmoConfig, GizmoMode, GizmoOrientation};
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
        context::{GuiContext, GuiViewManager, HiddenWindows},
        gizmo::draw_transform_gizmos,
        hotkeys,
        updater::{ChannelSelector, UpdateDownload},
        SelectionGizmoMode,
    },
    maplist::MapList,
    resources::Resources,
    updater::UpdateCheck,
    util::action::ActionList,
    ApplicationArgs,
};

pub struct AlkahestApp {
    pub window: winit::window::Window,
    pub event_loop: EventLoop<()>,

    pub gctx: Arc<GpuContext>,
    pub gui: GuiContext,
    pub resources: Resources,

    last_cursor_pos: Option<PhysicalPosition<f64>>,

    renderer: RendererShared,
    scratch_map: Scene,

    update_channel_gui: ChannelSelector,
    updater_gui: Option<UpdateDownload>,
}

impl AlkahestApp {
    pub fn new(
        event_loop: EventLoop<()>,
        icon: &winit::window::Icon,
        args: ApplicationArgs,
    ) -> Self {
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

        puffin::set_scopes_on(false);

        let gctx = Arc::new(GpuContext::create(&window).unwrap());
        let gui = GuiContext::create(&window, gctx.clone());
        let mut resources = Resources::default();
        resources.insert(GuiViewManager::with_default_views());
        resources.insert(InputState::default());
        resources.insert(CurrentActivity(args.activity));
        resources.insert(SelectedEntity::default());
        resources.insert(args);
        resources.insert(config!().renderer.clone());
        resources.insert(MapList::default());
        resources.insert(SelectionGizmoMode::default());
        resources.insert(HiddenWindows::default());
        resources.insert(ActionList::default());
        let renderer = Renderer::create(
            gctx.clone(),
            (window.inner_size().width, window.inner_size().height),
        )
        .unwrap();
        resources.insert(renderer.clone());
        let stringmap = Arc::new(GlobalStringmap::load());
        resources.insert(stringmap);

        let gizmo = Gizmo::new(GizmoConfig {
            modes: enum_set!(GizmoMode::Rotate | GizmoMode::Translate | GizmoMode::Scale),
            orientation: GizmoOrientation::Local,
            ..Default::default()
        });
        resources.insert(gizmo);

        resources
            .get_mut::<GuiViewManager>()
            .insert(ActivityBrowser::new(&resources.get::<StringMapShared>()));

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
            let map_name = get_map_name(maphash, &resources.get::<StringMapShared>())
                .unwrap_or_else(|_| format!("Unknown map {maphash}"));

            resources
                .get_mut::<MapList>()
                .add_map(&resources, map_name, maphash);
        }

        let mut node_filter_set = NodeFilterSet::default();
        for nf in NodeFilter::iter() {
            if !matches!(
                nf,
                NodeFilter::PlayerContainmentVolume
                    | NodeFilter::SlipSurfaceVolume
                    | NodeFilter::TurnbackBarrier
                    | NodeFilter::InstakillBarrier
                    | NodeFilter::Cubemap
                    | NodeFilter::NamedArea
            ) {
                node_filter_set.insert(nf);
            }
        }
        resources.insert(node_filter_set);

        Self {
            window,
            event_loop,
            gctx,
            gui,
            resources,
            last_cursor_pos: None,
            renderer,
            scratch_map: Scene::new(),
            update_channel_gui,
            updater_gui,
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
            ..
        } = self;

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
                                *last_cursor_pos = Some(PhysicalPosition::new(
                                    new_cursor_pos.0 as f64,
                                    new_cursor_pos.1 as f64,
                                ));

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
                    WindowEvent::MouseInput { .. } => {
                        let input = resources.get::<InputState>();
                        if input.mouse_left_clicked()
                            && !gui.egui.wants_pointer_input()
                            && !resources.get::<SelectedEntity>().changed_this_frame
                        {
                            if let Some(mouse_pos) = gui.egui.pointer_interact_pos() {
                                renderer.pickbuffer.request_selection(
                                    (mouse_pos.x as f64 * window.scale_factor()).round() as u32,
                                    (mouse_pos.y as f64 * window.scale_factor()).round() as u32,
                                );
                            }
                        }
                    }
                    WindowEvent::Resized(new_dims) => {
                        let _ = gui
                            .renderer
                            .resize_buffers(&gctx.swap_chain, || {
                                gctx.resize_swapchain(new_dims.width, new_dims.height);
                                HRESULT(0)
                            })
                            .expect("Failed to resize buffers");

                        renderer.resize_buffers(new_dims.width, new_dims.height);

                        resources.get_mut::<Camera>().set_viewport(Viewport {
                            size: glam::UVec2::new(new_dims.width, new_dims.height),
                            origin: glam::UVec2::ZERO,
                        });
                    }
                    WindowEvent::RedrawRequested => {
                        resources.get_mut::<SelectedEntity>().changed_this_frame = false;
                        renderer.data.lock().asset_manager.poll();
                        {
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
                                action_list.process(&resources);
                            }

                            resources
                                .get_mut::<Camera>()
                                .update(&resources.get::<InputState>(), renderer.delta_time as f32);

                            gctx.begin_frame();
                            let mut maps = resources.get_mut::<MapList>();
                            maps.update_maps(resources);

                            let map = maps.current_map().map(|m| &m.scene).unwrap_or(scratch_map);

                            renderer.render_world(&*resources.get::<Camera>(), map, resources);
                        }

                        unsafe {
                            renderer.gpu.context().OMSetRenderTargets(
                                Some(&[renderer.gpu.swapchain_target.read().clone()]),
                                None,
                            );
                        }

                        renderer.gpu.begin_event("interface_and_hud").scoped(|| {
                            gpu_event!(renderer.gpu, "egui");
                            gui.draw_frame(window, |ctx, ectx| {
                                hotkeys::process_hotkeys(ectx, resources);
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
                            });
                        });

                        window.pre_present_notify();
                        gctx.present(resources.get::<RendererSettings>().vsync);

                        window.request_redraw();
                        profiling::finish_frame!();

                        if !window.has_focus() {
                            // Slow the app down when it's not in focus
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }

                        if let Some(e) = renderer.pickbuffer.finish_request() {
                            let mut selected = resources.get_mut::<SelectedEntity>();
                            if !selected.changed_this_frame {
                                if e != u32::MAX {
                                    let maps = resources.get::<MapList>();
                                    if let Some(map) = maps.current_map() {
                                        selected
                                            .select(unsafe { map.scene.find_entity_from_id(e) });
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
