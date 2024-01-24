use std::cell::RefCell;
use std::mem::transmute;
use std::rc::Rc;
use std::sync::Arc;

use crate::render::DeviceContextSwapchain;
use crate::resources::Resources;
use crate::util::exe_relative_path;
use crate::util::image::{EguiPngLoader, Png};
use egui::epaint::ahash::HashMap;
use egui_directx11::DirectX11Renderer;
use egui_winit::EventResponse;
use itertools::Itertools;
use winit::event::WindowEvent;
use winit::window::Window;

pub trait Overlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        window: &Window,
        resources: &mut Resources,
        gui: &mut GuiContext<'_>,
    ) -> bool;
}

pub struct GuiManager {
    pub egui: egui::Context,
    pub integration: egui_winit::State,
    pub renderer: egui_directx11::DirectX11Renderer,
    overlays: Vec<Rc<RefCell<dyn Overlay>>>,
    dcs: Arc<DeviceContextSwapchain>,
    resources: GuiResources,

    show_ui: bool,
}

impl GuiManager {
    pub fn create(window: &Window, dcs: Arc<DeviceContextSwapchain>) -> Self {
        let egui = egui::Context::default();

        egui.add_image_loader(Arc::new(EguiPngLoader::default()));

        if let Ok(Ok(data)) = std::fs::read_to_string(exe_relative_path("egui.ron"))
            .map(|s| ron::from_str::<egui::Memory>(&s))
        {
            info!("Loaded egui state from egui.ron");
            egui.memory_mut(|memory| *memory = data);
        }

        let integration = egui_winit::State::new(
            egui.clone(),
            egui::ViewportId::default(),
            window,
            Some(window.scale_factor() as f32),
            Some(8192),
        );

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "materialdesignicons".into(),
            egui::FontData::from_static(include_bytes!("../../materialdesignicons-webfont.ttf")),
        );
        fonts.font_data.insert(
            "Destiny_Keys".into(),
            egui::FontData::from_static(include_bytes!("../../Destiny_Keys.otf")),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(1, "materialdesignicons".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(2, "Destiny_Keys".to_owned());

        egui.set_fonts(fonts);

        let renderer = egui_directx11::DirectX11Renderer::init_from_swapchain(unsafe {
            transmute(&dcs.swap_chain)
        })
        .expect("Failed to initialize egui renderer");

        GuiManager {
            resources: GuiResources::load(&egui),
            egui,
            integration,
            renderer,
            overlays: vec![],
            dcs,
            show_ui: true,
        }
    }

    pub fn add_overlay(&mut self, overlay: Rc<RefCell<dyn Overlay>>) {
        self.overlays.push(overlay);
    }

    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> EventResponse {
        self.integration.on_window_event(window, event)
    }

    pub fn draw_frame<MF>(&mut self, window: Arc<Window>, resources: &mut Resources, misc_draw: MF)
    where
        MF: FnOnce(&egui::Context, &mut Resources),
    {
        if self.egui.input_mut(|i| {
            i.consume_key(
                egui::Modifiers {
                    alt: false,
                    ctrl: true,
                    shift: true,
                    mac_cmd: false,
                    command: false,
                },
                egui::Key::H,
            )
        }) {
            self.show_ui = !self.show_ui;
        }

        let input = self.integration.take_egui_input(&window);

        let output = self
            .renderer
            .paint(
                unsafe { transmute(&self.dcs.swap_chain) },
                input,
                &self.egui,
                window.scale_factor() as f32,
                |integration, ctx| {
                    if self.show_ui {
                        for overlay in self.overlays.iter() {
                            overlay.as_ref().borrow_mut().draw(
                                ctx,
                                &window,
                                resources,
                                &mut GuiContext {
                                    icons: &self.resources,
                                    integration,
                                },
                            );
                        }

                        let viewer_keys = resources
                            .get::<ViewerWindows>()
                            .map(|v| v.0.keys().cloned().collect_vec())
                            .unwrap_or_default();

                        // Extract each viewer window individually so that we can pass resources into it, adding it back in if the viewer returns true
                        for k in viewer_keys {
                            let mut viewer = resources
                                .get_mut::<ViewerWindows>()
                                .unwrap()
                                .0
                                .remove(&k)
                                .unwrap();

                            if viewer.draw(
                                ctx,
                                &window,
                                resources,
                                &mut GuiContext {
                                    icons: &self.resources,
                                    integration,
                                },
                            ) {
                                resources
                                    .get_mut::<ViewerWindows>()
                                    .unwrap()
                                    .0
                                    .insert(k, viewer);
                            }
                        }

                        misc_draw(ctx, resources);
                    }
                },
            )
            .unwrap();

        self.integration
            .handle_platform_output(&window, output.platform_output)
    }
}

impl Drop for GuiManager {
    fn drop(&mut self) {
        match self.egui.memory(ron::to_string) {
            Ok(memory) => {
                if let Err(e) = std::fs::write(exe_relative_path("egui.ron"), memory) {
                    error!("Failed to write egui state: {e}");
                }
            }
            Err(e) => {
                error!("Failed to serialize egui state: {e}");
            }
        };
    }
}

pub struct GuiContext<'a> {
    pub icons: &'a GuiResources,
    pub integration: &'a mut DirectX11Renderer,
}

pub struct GuiResources {
    pub icon_havok: egui::TextureHandle,
}

impl GuiResources {
    pub fn load(ctx: &egui::Context) -> Self {
        let img = Png::from_bytes(include_bytes!("../../assets/icons/havok_dark_256.png")).unwrap();
        let icon_havok = ctx.load_texture(
            "Havok 64x64",
            egui::ImageData::Color(
                egui::ColorImage::from_rgba_premultiplied(img.dimensions, &img.data).into(),
            ),
            egui::TextureOptions {
                magnification: egui::TextureFilter::Linear,
                minification: egui::TextureFilter::Linear,
            },
        );

        Self { icon_havok }
    }
}

#[derive(Default)]
pub struct ViewerWindows(pub HashMap<String, Box<dyn Overlay>>);
