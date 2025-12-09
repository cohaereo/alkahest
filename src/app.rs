#![warn(rust_2018_idioms)]
#![deny(clippy::correctness, clippy::suspicious, clippy::complexity)]
#![allow(clippy::collapsible_else_if, clippy::missing_transmute_annotations)]

use std::{rc::Rc, sync::Arc, time::Instant};

use alkahest_render::{
    Gpu, Renderer,
    gpu::{command_list::CommandList, spinner::FullscreenSpinner},
    util::fps_histogram::FrametimeHistogram,
};
use sdl3::video::Window;

use crate::{cli::AppArgs, ui::Gui};

pub struct App {
    pub sdl: Rc<sdl3::Sdl>,
    pub window: Rc<Window>,
    pub gpu: Arc<Gpu>,
    pub renderer: Arc<Renderer>,
    pub gui: Gui,
    pub running: bool,

    spinner: FullscreenSpinner,
    last_frame_time: Instant,
    start_time: Instant,
    frametime_histogram: FrametimeHistogram,
}

impl App {
    pub fn new(sdl: Rc<sdl3::Sdl>, window: Rc<Window>, _args: AppArgs) -> anyhow::Result<Self> {
        let gpu = Arc::new(Gpu::create(&window)?);
        let renderer = Arc::new(Renderer::new(gpu.clone())?);
        Renderer::set_instance(renderer.clone());

        Ok(Self {
            spinner: FullscreenSpinner::create(&renderer.gpu)?,
            renderer,
            gui: Gui::new(&gpu, sdl.clone(), window.clone())?,
            sdl,
            window,
            gpu,
            running: true,

            last_frame_time: Instant::now(),
            start_time: Instant::now(),
            frametime_histogram: FrametimeHistogram::new(10),
        })
    }

    pub fn handle_event(&mut self, event: sdl3::event::Event) {
        #[allow(clippy::single_match, clippy::collapsible_match)]
        match &event {
            sdl3::event::Event::Quit { .. } => {
                self.running = false;
            }
            sdl3::event::Event::Window { win_event, .. } => match win_event {
                &sdl3::event::WindowEvent::Resized(new_width, new_height) => {
                    self.gui
                        .egui_d3d11
                        .resize_buffers(&self.renderer.gpu, || {
                            self.renderer
                                .resize_swapchain((new_width as u32, new_height as u32));
                            Ok(())
                        })
                        .ok();
                }
                sdl3::event::WindowEvent::CloseRequested => {
                    self.running = false;
                }
                _ => {}
            },
            _ => {}
        };

        self.gui
            .egui_sdl3
            .handle_event(&event, &self.sdl, &self.sdl.video().unwrap());
    }

    #[profiling::function]
    pub fn render(&mut self, _event_pump: &sdl3::EventPump) {
        let frame_start = std::time::Instant::now();
        let refresh_rate = if !self.window.has_input_focus() && !self.window.has_mouse_focus() {
            10.0
        } else {
            self.window
                .get_display()
                .and_then(|d| d.get_mode())
                .map(|m| m.refresh_rate)
                .unwrap_or(60.0)
        };
        let frame_end =
            frame_start + std::time::Duration::from_millis((1000.0 / refresh_rate) as u64);

        let delta_time = (frame_start - self.last_frame_time).as_secs_f32();
        self.last_frame_time = frame_start;

        self.frametime_histogram.push(delta_time);

        self.renderer.begin_frame();

        let gpu = &self.renderer.gpu;
        let mut cmd = CommandList::from_device_context(gpu, gpu.context().clone());
        subsecond::call(|| {
            self.gui.draw(&mut cmd);
        });

        let vsync = false;
        self.renderer.present_frame(vsync);
        if !vsync {
            spin_sleep::sleep_until(frame_end);
        }

        profiling::finish_frame!();
    }
}
