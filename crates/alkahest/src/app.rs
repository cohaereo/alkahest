use std::sync::Arc;

use alkahest_renderer::{
    camera::{Camera, Viewport},
    ecs::{resources::SelectedEntity, Scene},
    gpu::GpuContext,
    gpu_event,
    input::InputState,
    renderer::{Renderer, RendererSettings, RendererShared},
};
use anyhow::Context;
use egui::{Key, KeyboardShortcut, Modifiers, Widget};
use glam::Vec2;
use windows::core::HRESULT;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{MouseScrollDelta, WindowEvent},
    event_loop::EventLoop,
    platform::run_on_demand::EventLoopExtRunOnDemand,
};

use crate::{
    config,
    data::text::{GlobalStringmap, StringMapShared},
    gui::{
        activity_select::{get_map_name, ActivityBrowser, CurrentActivity},
        context::{GuiContext, GuiViewManager},
    },
    maplist::MapList,
    resources::Resources,
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
    map_placeholder: Scene,
}

impl AlkahestApp {
    pub fn new(
        event_loop: EventLoop<()>,
        icon: &winit::window::Icon,
        args: crate::ApplicationArgs,
    ) -> Self {
        let window = winit::window::WindowBuilder::new()
            .with_title("Alkahest")
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
        let renderer = Renderer::create(
            gctx.clone(),
            (window.inner_size().width, window.inner_size().height),
        )
        .unwrap();
        resources.insert(renderer.clone());
        let stringmap = Arc::new(GlobalStringmap::load().expect("Failed to load global strings"));
        resources.insert(stringmap);

        resources
            .get_mut::<GuiViewManager>()
            .insert(ActivityBrowser::new(&resources.get::<StringMapShared>()));

        let camera = Camera::new_fps(Viewport {
            size: glam::UVec2::new(1920, 1080),
            origin: glam::UVec2::new(0, 0),
        });
        resources.insert(camera);

        if let Some(maphash) = resources.get::<ApplicationArgs>().map {
            let map_name = get_map_name(maphash, &resources.get::<StringMapShared>())
                .unwrap_or_else(|_| format!("Unknown map {maphash}"));

            resources.get_mut::<MapList>().add_map(map_name, maphash);
        }

        Self {
            window,
            event_loop,
            gctx,
            gui,
            resources,
            last_cursor_pos: None,
            renderer,
            map_placeholder: Scene::new(),
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
            map_placeholder,
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
                        if let Some(ref mut p) = last_cursor_pos {
                            let delta = (position.x - p.x, position.y - p.y);
                            let input = resources.get::<InputState>();
                            if (input.mouse_left() | input.mouse_middle())
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
                    WindowEvent::Resized(new_dims) => {
                        let _ = gui
                            .renderer
                            .resize_buffers(&gctx.swap_chain, || {
                                gctx.resize_swapchain(new_dims.width, new_dims.height);
                                HRESULT(0)
                            })
                            .expect("Failed to resize buffers");

                        renderer
                            .data
                            .lock()
                            .gbuffers
                            .resize((new_dims.width, new_dims.height))
                            .expect("Failed to resize GBuffer");
                        resources.get_mut::<Camera>().set_viewport(Viewport {
                            size: glam::UVec2::new(new_dims.width, new_dims.height),
                            origin: glam::UVec2::ZERO,
                        });
                    }
                    WindowEvent::RedrawRequested => {
                        renderer.data.lock().asset_manager.poll();
                        let render_settings = resources.get::<RendererSettings>();

                        if gui.input_mut(|i| {
                            i.consume_shortcut(&KeyboardShortcut::new(Modifiers::ALT, Key::Enter))
                        }) {
                            if window.fullscreen().is_some() {
                                window.set_fullscreen(None);
                            } else {
                                window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(
                                    window.current_monitor(),
                                )));
                            }

                            config::with_mut(|c| {
                                c.window.fullscreen = window.fullscreen().is_some();
                            });
                        }

                        resources
                            .get_mut::<Camera>()
                            .update(&resources.get::<InputState>(), renderer.delta_time as f32);

                        gctx.begin_frame();
                        let mut maps = resources.get_mut::<MapList>();
                        maps.update_maps(resources);

                        let map = maps
                            .current_map()
                            .map(|m| &m.scene)
                            .unwrap_or(map_placeholder);

                        renderer.render_world(&*resources.get::<Camera>(), map);

                        unsafe {
                            renderer.gpu.context().OMSetRenderTargets(
                                Some(&[renderer.gpu.swapchain_target.read().clone()]),
                                None,
                            );
                        }

                        drop(maps);
                        drop(render_settings);

                        renderer.gpu.begin_event("interface_and_hud").scoped(|| {
                            gpu_event!(renderer.gpu, "egui");
                            gui.draw_frame(window, |ctx, ectx| {
                                let mut gui_views = resources.get_mut::<GuiViewManager>();
                                gui_views.draw(ectx, window, resources, ctx);

                                if !gui_views.hide_views {
                                    puffin_egui::profiler_window(ectx);

                                    egui::Window::new("SSAO Settings").show(ectx, |ui| {
                                        let ssao_data = renderer.ssao.scope.data();
                                        ui.horizontal(|ui| {
                                            ui.label("Radius");
                                            egui::DragValue::new(&mut ssao_data.radius)
                                                .speed(0.01)
                                                .clamp_range(0.0..=10.0)
                                                .suffix("m")
                                                .ui(ui);
                                        });

                                        ui.horizontal(|ui| {
                                            ui.label("Bias");
                                            egui::DragValue::new(&mut ssao_data.bias)
                                                .speed(0.01)
                                                .clamp_range(0.0..=10.0)
                                                .suffix("m")
                                                .ui(ui);
                                        });
                                    });
                                }
                            });
                        });

                        window.pre_present_notify();
                        gctx.present(resources.get::<RendererSettings>().vsync);

                        window.request_redraw();
                        profiling::finish_frame!();
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
