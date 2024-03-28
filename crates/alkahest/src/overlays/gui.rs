use std::{cell::RefCell, mem::transmute, rc::Rc, sync::Arc};

use egui::epaint::ahash::HashMap;
use egui_directx11::DirectX11Renderer;
use egui_winit::EventResponse;
use itertools::Itertools;
use winit::{event::WindowEvent, window::Window};

use crate::{
    config::APP_DIRS,
    render::DeviceContextSwapchain,
    resources::Resources,
    util::image::{EguiPngLoader, Png},
};

pub trait Overlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        window: &Window,
        resources: &mut Resources,
        gui: &mut GuiContext<'_>,
    ) -> bool;

    fn dispose(
        &mut self,
        _ctx: &egui::Context,
        _resources: &mut Resources,
        _gui: &mut GuiContext<'_>,
    ) {
    }
}

#[derive(PartialEq)]
pub enum PreDrawResult {
    /// Continue drawing the rest of the UI
    Continue,
    /// Don't draw the rest of the UI
    Stop,
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

        if let Ok(Ok(data)) = std::fs::read_to_string(APP_DIRS.config_dir().join("egui.ron"))
            .map(|s| ron::from_str::<egui::Memory>(&s))
        {
            info!("Loaded egui state from egui.ron");
            egui.memory_mut(|memory| *memory = data);
        }

        let integration = egui_winit::State::new(
            egui::ViewportId::default(),
            window,
            Some(window.scale_factor() as f32),
            Some(8192),
        );

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Inter-Medium".into(),
            egui::FontData::from_static(include_bytes!("../../assets/fonts/Inter-Medium.ttf")),
        );

        fonts.font_data.insert(
            "materialdesignicons".into(),
            egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/materialdesignicons-webfont.ttf"
            )),
        );
        fonts.font_data.insert(
            "Destiny_Keys".into(),
            egui::FontData::from_static(include_bytes!("../../assets/fonts/Destiny_Keys.otf")),
        );

        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "Inter-Medium".into());
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
        egui.set_style(style::style());

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

    pub fn handle_event(&mut self, event: &WindowEvent<'_>) -> EventResponse {
        self.integration.on_window_event(&self.egui, event)
    }

    pub fn draw_frame<PF, MF>(
        &mut self,
        window: Arc<Window>,
        resources: &mut Resources,
        pre_draw: PF,
        misc_draw: MF,
    ) where
        PF: FnOnce(&egui::Context, &mut Resources) -> PreDrawResult,
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
                    if pre_draw(ctx, resources) == PreDrawResult::Stop {
                        return;
                    }

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

                        // Extract each viewer window individually so that we can pass resources
                        // into it, adding it back in if the viewer returns true
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
                            } else {
                                viewer.dispose(
                                    ctx,
                                    resources,
                                    &mut GuiContext {
                                        icons: &self.resources,
                                        integration,
                                    },
                                );
                            }
                        }

                        misc_draw(ctx, resources);
                    }
                },
            )
            .unwrap();

        self.integration
            .handle_platform_output(&window, &self.egui, output.platform_output)
    }
}

impl Drop for GuiManager {
    fn drop(&mut self) {
        match self.egui.memory(ron::to_string) {
            Ok(memory) => {
                if let Err(e) = std::fs::write(APP_DIRS.config_dir().join("egui.ron"), memory) {
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

#[derive(Default)]
pub struct HiddenWindows {
    pub texture_dumper: bool,
    pub tag_dumper: bool,
}

mod style {
    // Generated by egui-themer (https://github.com/grantshandy/egui-themer).

    use egui::{
        epaint::Shadow,
        style::{Interaction, Margin, Selection, Spacing, WidgetVisuals, Widgets},
        Color32, Rounding, Stroke, Style, Vec2, Visuals,
    };

    pub fn style() -> Style {
        Style {
            // override the text styles here:
            // override_text_style: Option<TextStyle>

            // override the font id here:
            // override_font_id: Option<FontId>

            // set your text styles here:
            // text_styles: BTreeMap<TextStyle, FontId>,

            // set your drag value text style:
            // drag_value_text_style: TextStyle,
            spacing: Spacing {
                item_spacing: Vec2 { x: 8.0, y: 6.0 },
                window_margin: Margin {
                    left: 6.0,
                    right: 6.0,
                    top: 6.0,
                    bottom: 6.0,
                },
                button_padding: Vec2 { x: 9.0, y: 5.0 },
                menu_margin: Margin {
                    left: 6.0,
                    right: 6.0,
                    top: 6.0,
                    bottom: 6.0,
                },
                indent: 18.0,
                interact_size: Vec2 { x: 40.0, y: 20.0 },
                slider_width: 100.0,
                combo_width: 100.0,
                text_edit_width: 280.0,
                icon_width: 14.0,
                icon_width_inner: 8.0,
                icon_spacing: 4.0,
                tooltip_width: 600.0,
                indent_ends_with_horizontal_line: true,
                combo_height: 200.0,
                // scroll_bar_width: 8.0,
                // scroll_handle_min_length: 12.0,
                // scroll_bar_inner_margin: 4.0,
                // scroll_bar_outer_margin: 0.0,
                ..Default::default()
            },
            interaction: Interaction {
                resize_grab_radius_side: 5.0,
                resize_grab_radius_corner: 10.0,
                show_tooltips_only_when_still: true,
                ..Default::default()
            },
            visuals: Visuals {
                dark_mode: true,
                override_text_color: None,
                widgets: Widgets {
                    noninteractive: WidgetVisuals {
                        bg_fill: Color32::from_rgba_premultiplied(60, 60, 60, 128),
                        weak_bg_fill: Color32::from_rgba_premultiplied(38, 38, 38, 128),
                        bg_stroke: Stroke {
                            width: 0.25,
                            color: Color32::from_rgba_premultiplied(255, 255, 255, 255),
                        },
                        rounding: Rounding {
                            nw: 6.0,
                            ne: 6.0,
                            sw: 6.0,
                            se: 6.0,
                        },
                        fg_stroke: Stroke {
                            width: 1.5,
                            color: Color32::from_rgba_premultiplied(180, 180, 180, 255),
                        },
                        expansion: 0.0,
                    },
                    inactive: WidgetVisuals {
                        bg_fill: Color32::from_rgba_premultiplied(60, 60, 60, 255),
                        weak_bg_fill: Color32::from_rgba_premultiplied(38, 38, 38, 255),
                        bg_stroke: Stroke {
                            width: 0.25,
                            color: Color32::from_rgba_premultiplied(255, 255, 255, 255),
                        },
                        rounding: Rounding {
                            nw: 6.0,
                            ne: 6.0,
                            sw: 6.0,
                            se: 6.0,
                        },
                        fg_stroke: Stroke {
                            width: 1.5,
                            color: Color32::from_rgba_premultiplied(180, 180, 180, 255),
                        },
                        expansion: 0.0,
                    },
                    hovered: WidgetVisuals {
                        bg_fill: Color32::from_rgba_premultiplied(70, 70, 70, 255),
                        weak_bg_fill: Color32::from_rgba_premultiplied(70, 70, 70, 255),
                        bg_stroke: Stroke {
                            width: 0.5,
                            color: Color32::from_rgba_premultiplied(150, 150, 150, 255),
                        },
                        rounding: Rounding {
                            nw: 6.0,
                            ne: 6.0,
                            sw: 6.0,
                            se: 6.0,
                        },
                        fg_stroke: Stroke {
                            width: 1.5,
                            color: Color32::from_rgba_premultiplied(240, 240, 240, 255),
                        },
                        expansion: 1.0,
                    },
                    active: WidgetVisuals {
                        bg_fill: Color32::from_rgba_premultiplied(55, 55, 55, 255),
                        weak_bg_fill: Color32::from_rgba_premultiplied(55, 55, 55, 255),
                        bg_stroke: Stroke {
                            width: 0.25,
                            color: Color32::from_rgba_premultiplied(255, 255, 255, 255),
                        },
                        rounding: Rounding {
                            nw: 6.0,
                            ne: 6.0,
                            sw: 6.0,
                            se: 6.0,
                        },
                        fg_stroke: Stroke {
                            width: 1.5,
                            color: Color32::from_rgba_premultiplied(255, 255, 255, 255),
                        },
                        expansion: 0.0,
                    },
                    open: WidgetVisuals {
                        bg_fill: Color32::from_rgba_premultiplied(27, 27, 27, 255),
                        weak_bg_fill: Color32::from_rgba_premultiplied(27, 27, 27, 255),
                        bg_stroke: Stroke {
                            width: 0.25,
                            color: Color32::from_rgba_premultiplied(60, 60, 60, 255),
                        },
                        rounding: Rounding {
                            nw: 6.0,
                            ne: 6.0,
                            sw: 6.0,
                            se: 6.0,
                        },
                        fg_stroke: Stroke {
                            width: 1.5,
                            color: Color32::from_rgba_premultiplied(210, 210, 210, 255),
                        },
                        expansion: 0.0,
                    },
                },
                selection: Selection {
                    bg_fill: Color32::from_rgba_premultiplied(23, 95, 93, 255),
                    stroke: Stroke {
                        width: 5.4,
                        color: Color32::from_rgba_premultiplied(255, 255, 255, 255),
                    },
                },
                hyperlink_color: Color32::from_rgba_premultiplied(90, 170, 255, 255),
                faint_bg_color: Color32::from_rgba_premultiplied(5, 5, 5, 0),
                extreme_bg_color: Color32::from_rgba_premultiplied(10, 10, 10, 255),
                code_bg_color: Color32::from_rgba_premultiplied(64, 64, 64, 255),
                warn_fg_color: Color32::from_rgba_premultiplied(255, 143, 0, 255),
                error_fg_color: Color32::from_rgba_premultiplied(255, 0, 0, 255),
                window_rounding: Rounding {
                    nw: 6.0,
                    ne: 6.0,
                    sw: 6.0,
                    se: 6.0,
                },
                window_shadow: Shadow {
                    extrusion: 16.0,
                    color: Color32::from_rgba_premultiplied(0, 0, 0, 96),
                },
                window_fill: Color32::from_rgba_premultiplied(6, 5, 7, 255),
                window_stroke: Stroke {
                    width: 1.0,
                    color: Color32::from_rgba_premultiplied(21, 21, 21, 255),
                },
                menu_rounding: Rounding {
                    nw: 6.0,
                    ne: 6.0,
                    sw: 6.0,
                    se: 6.0,
                },
                panel_fill: Color32::from_rgba_premultiplied(27, 27, 27, 255),
                popup_shadow: Shadow {
                    extrusion: 16.0,
                    color: Color32::from_rgba_premultiplied(0, 0, 0, 96),
                },
                resize_corner_size: 12.0,
                // text_cursor_width: 2.0,
                text_cursor_preview: false,
                clip_rect_margin: 3.0,
                button_frame: true,
                collapsing_header_frame: false,
                indent_has_left_vline: true,
                striped: false,
                slider_trailing_fill: false,
                ..Default::default()
            },
            animation_time: 0.083333336,
            explanation_tooltips: false,
            ..Default::default()
        }
    }
}
