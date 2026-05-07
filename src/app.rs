#![warn(rust_2018_idioms)]
#![deny(clippy::correctness, clippy::suspicious, clippy::complexity)]
#![allow(clippy::collapsible_else_if, clippy::missing_transmute_annotations)]

use std::{
    io::{Cursor, Seek},
    rc::Rc,
    str::FromStr,
    sync::Arc,
    time::Instant,
};

use ahash::HashMap;
use alkahest_core::job::SCHEDULER;
use alkahest_data::{
    strings::{StringContainer, StringContainerShared},
    tag::WideHash,
};
use alkahest_render::{
    Gpu, Renderer,
    gpu::{command_list::CommandList, spinner::FullscreenSpinner},
    util::fps_histogram::FrametimeHistogram,
};
use anyhow::Context;
use parking_lot::RwLock;
use sdl3::video::Window;
use tiger_parse::TigerReadable;
use tiger_pkg::{TagHash, package_manager};

use crate::{
    cli::AppArgs,
    config::AppConfig,
    ui::{
        Gui,
        tabs::{Tab, map::MapTab, test_scene::TestSceneTab},
    },
};

pub struct App {
    pub sdl: Rc<sdl3::Sdl>,
    pub _window: Rc<Window>,
    pub _gpu: Arc<Gpu>,
    pub renderer: Arc<Renderer>,
    pub gui: Gui,
    pub running: bool,

    shared_state: Arc<SharedState>,

    _spinner: FullscreenSpinner,
    last_frame_time: Instant,
    frametime_histogram: FrametimeHistogram,
}

impl App {
    pub fn new(sdl: Rc<sdl3::Sdl>, window: Rc<Window>, args: AppArgs) -> anyhow::Result<Self> {
        let gpu = Arc::new(Gpu::create(&window).context("Failed to create GPU")?);
        let renderer = Arc::new(Renderer::new(gpu.clone()).context("Failed to create renderer")?);
        Renderer::set_instance(renderer.clone());

        let shared_state: Arc<SharedState> = SharedState::new()
            .context("Failed to create shared state")?
            .into();
        let mut gui = Gui::new(&gpu, sdl.clone(), window.clone())?;
        if let Some(map_hash) = args.open_map.as_ref() {
            match TagHash::from_str(map_hash) {
                Ok(tag) => match MapTab::new(tag, String::new(), &shared_state) {
                    Ok(tab) => gui.add_tab(Tab::Map(tab)),
                    Err(e) => error!("Failed to open map tab for {}: {:?}", map_hash, e),
                },
                Err(e) => {
                    error!("Failed to parse map hash {}: {:?}", map_hash, e);
                }
            };
        }

        if args.test_scene {
            gui.add_tab(Tab::TestScene(
                TestSceneTab::new(&shared_state).context("failed to create test scene")?,
            ));
        }

        Ok(Self {
            _spinner: FullscreenSpinner::create(&renderer.gpu)?,
            renderer,
            gui,
            sdl,
            _window: window,
            _gpu: gpu,
            running: true,

            shared_state,

            last_frame_time: Instant::now(),
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
        let delta_time = self.last_frame_time.elapsed().as_secs_f32();
        self.last_frame_time = std::time::Instant::now();

        self.frametime_histogram.push(delta_time);

        self.renderer.begin_frame();

        let gpu = &self.renderer.gpu;
        let mut cmd = CommandList::from_device_context(gpu, gpu.context().clone());
        subsecond::call(|| {
            self.gui.draw(&mut cmd, &self.shared_state);
        });

        self.renderer
            .present_frame(self.shared_state.config.read().vsync);

        let config = self.shared_state.config.read();
        if config.framelimiter_enabled {
            let target_frame_delta = 1.0 / config.framerate_limit as f32;
            while self.last_frame_time.elapsed().as_secs_f32() < target_frame_delta {
                std::hint::spin_loop();
            }
        }

        profiling::finish_frame!();
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.shared_state.save_config().ok();
        SCHEDULER.shutdown();
    }
}

pub struct SharedState {
    pub strings: StringContainerShared,
    pub strings_by_package: HashMap<String, StringContainer>,
    pub config: RwLock<AppConfig>,
}

impl SharedState {
    pub fn new() -> anyhow::Result<Self> {
        let mut strings_by_package = HashMap::default();
        for (name, tag) in package_manager().get_named_tags_by_class(0x80808E8B) {
            let Ok(data) = package_manager().read_tag(tag) else {
                continue;
            };
            let mut cur = Cursor::new(data);
            cur.seek(std::io::SeekFrom::Start(0x10))?;
            let hash = WideHash::read_ds(&mut cur)?;
            if hash.is_none() {
                continue;
            }
            strings_by_package.insert(name, StringContainer::load(hash)?);
        }

        let s = Self {
            strings: StringContainer::load_all_global().into(),
            strings_by_package,
            config: RwLock::new(AppConfig::default()),
        };
        if let Err(e) = s.load_config() {
            warn!("Failed to load config: {:?}", e);
        }

        Ok(s)
    }

    pub fn load_config(&self) -> anyhow::Result<()> {
        let exe_path = std::env::current_exe()?.parent().unwrap().to_path_buf();
        let config_path = exe_path.join("config.toml");
        if config_path.exists() {
            let config_str = std::fs::read_to_string(&config_path)?;
            let config: AppConfig = toml::from_str(&config_str)?;
            *self.config.write() = config;
        }

        Ok(())
    }

    pub fn save_config(&self) -> anyhow::Result<()> {
        let exe_path = std::env::current_exe()?.parent().unwrap().to_path_buf();
        let config_path = exe_path.join("config.toml");
        let config_str = toml::to_string_pretty(&*self.config.read())?;
        std::fs::write(&config_path, config_str)?;

        Ok(())
    }

    pub fn get_string(&self, hash: u32) -> String {
        self.strings.get(hash)
    }

    pub fn get_string_by_package(&self, package: &str, hash: u32) -> String {
        self.strings_by_package
            .get(package)
            .and_then(|s| s.try_get(hash))
            .unwrap_or_else(|| self.get_string(hash))
    }
}
