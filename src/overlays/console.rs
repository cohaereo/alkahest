use crate::camera::FpsCamera;
use crate::ecs::components::EntityModel;
use crate::ecs::transform::Transform;
use crate::entity::SEntityModel;
use crate::map::MapDataList;
use crate::material::Technique;
use crate::overlays::gui::Overlay;
use crate::packages::package_manager;
use crate::render::bytecode::opcodes::TfxBytecodeOp;
use crate::render::dcs::DcsShared;
use crate::render::EntityRenderer;

use crate::render::renderer::{Renderer, RendererShared};
use crate::resources::Resources;
use crate::structure::ExtendedHash;

use anyhow::Context;
use binrw::BinReaderExt;
use destiny_pkg::{TagHash, TagHash64};
use egui::{Color32, RichText, TextStyle};
use glam::Vec3;
use itertools::Itertools;
use lazy_static::lazy_static;
use ringbuffer::{AllocRingBuffer, RingBuffer};
use std::fmt::Debug;
use std::io::Cursor;
use std::sync::Arc;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::Layer;
use winit::window::Window;

use super::gui::ViewerWindows;
use super::texture_viewer::TextureViewer;

// ! Do NOT swap this RwLock to our own implementation, as it will cause infinite recursion
lazy_static! {
    static ref MESSAGE_BUFFER: Arc<parking_lot::RwLock<AllocRingBuffer<CapturedEvent>>> =
        Arc::new(parking_lot::RwLock::new(AllocRingBuffer::new(8192)));
}

/// Tracing layer to capture events
pub struct ConsoleLogLayer;

struct ConsoleLogVisitor {
    fields: Vec<(String, String)>,
}

impl Visit for ConsoleLogVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.fields
            .push((field.name().to_string(), format!("{value:?}")))
    }
}

struct CapturedEvent {
    level: Level,
    target: String,
    message: String,
}

impl<S> Layer<S> for ConsoleLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = ConsoleLogVisitor { fields: vec![] };

        event.record(&mut visitor);
        let mut message = None;
        for (f, v) in visitor.fields {
            if f.as_str() == "message" {
                message = Some(v);
            }
        }

        if let Some(message) = message {
            MESSAGE_BUFFER.write().push(CapturedEvent {
                level: *event.metadata().level(),
                target: event.metadata().target().to_string(),
                message,
            })
        }
    }
}

pub struct ConsoleOverlay {
    pub command_buffer: String,
    pub autoscroll: bool,
    pub open: bool,
}

impl Default for ConsoleOverlay {
    fn default() -> Self {
        Self {
            command_buffer: "".to_string(),
            autoscroll: true,
            open: false,
        }
    }
}

impl Overlay for ConsoleOverlay {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &mut Resources,
        _gui: super::gui::GuiContext<'_>,
    ) -> bool {
        let request_focus = if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.open = true;
            true
        } else {
            false
        };

        let response = egui::Window::new("Console")
            .open(&mut self.open)
            .collapsible(false)
            .show(ctx, |ui| {
                {
                    let c = MESSAGE_BUFFER.read();
                    let text_style = TextStyle::Monospace;
                    let row_height = ui.text_style_height(&text_style);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .max_height(_window.inner_size().height as f32 * 0.60)
                        .stick_to_bottom(true)
                        .show_rows(ui, row_height, c.len(), |ui, row_range| {
                            for row in row_range {
                                let event = &c[row as isize];
                                let level_color = match event.level {
                                    Level::TRACE => [0.8, 0.4, 0.8],
                                    Level::DEBUG => [0.35, 0.35, 1.0],
                                    Level::INFO => [0.25, 1.0, 0.25],
                                    Level::WARN => [1.0, 1.0, 0.15],
                                    Level::ERROR => [1.0, 0.15, 0.15],
                                };
                                let level_color = Color32::from_rgb(
                                    (level_color[0] * 255.0) as u8,
                                    (level_color[1] * 255.0) as u8,
                                    (level_color[2] * 255.0) as u8,
                                );

                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:5} ", event.level))
                                            .color(level_color)
                                            .monospace(),
                                    );
                                    ui.label(
                                        RichText::new(format!("{}: ", event.target))
                                            .color(Color32::GRAY)
                                            .monospace(),
                                    );
                                    ui.label(RichText::new(&event.message).monospace());
                                });
                            }
                        });
                }

                ui.horizontal(|ui| {
                    if egui::TextEdit::singleline(&mut self.command_buffer)
                        .id(egui::Id::new("console_input_line"))
                        .show(ui)
                        .response
                        .lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        let cmd = self.command_buffer.split(' ').collect_vec();
                        if !cmd.is_empty() {
                            let command = cmd[0];
                            let args = &cmd[1..];

                            execute_command(command, args, resources);
                        }

                        self.command_buffer.clear();
                        ctx.memory_mut(|m| m.request_focus(egui::Id::new("console_input_line")));
                    }
                });
            });

        if request_focus {
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("console_input_line")));
        }

        if let Some(response) = response {
            if response.response.has_focus() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.open = false;
            }
        }

        true
    }
}

fn execute_command(command: &str, args: &[&str], resources: &Resources) {
    match command.to_lowercase().as_str() {
        "goto" => {
            if args.len() != 3 {
                error!("Too few/many arguments, expected 3, got {}", args.len());
                return;
            }

            let parsed_pos: anyhow::Result<Vec3> = (|| {
                let x = str::parse(args[0])?;
                let y = str::parse(args[1])?;
                let z = str::parse(args[2])?;

                Ok(Vec3::new(x, y, z))
            })();

            match parsed_pos {
                Ok(new_pos) => {
                    let mut camera = resources.get_mut::<FpsCamera>().unwrap();
                    camera.position = new_pos;
                    info!("Teleported to {} {} {}", new_pos.x, new_pos.y, new_pos.z);
                }
                Err(e) => {
                    error!("Invalid coordinates: {e}");
                }
            }
        }
        "goto.raw" => {
            if args.len() != 1 || (args[0].len() != 24 && args[0].len() != 32) {
                error!(
                    "Missing/too short argument, expected 24/32 hex bytes, got {} args",
                    args.len()
                );
                return;
            }

            let Ok(raw_data) = hex::decode(args[0]) else {
                error!("Invalid hex position data");
                return;
            };

            let mut c = Cursor::new(raw_data);
            let new_pos = Vec3::new(
                c.read_le().unwrap(),
                c.read_le().unwrap(),
                c.read_le().unwrap(),
            );

            let mut camera = resources.get_mut::<FpsCamera>().unwrap();
            camera.position = new_pos;
            info!("Teleported to {} {} {}", new_pos.x, new_pos.y, new_pos.z);
        }
        "open.tex" | "open.texture" => {
            if args.len() != 1 {
                error!("Missing tag argument, expected 32/64-bit tag");
                return;
            }

            let tag_parsed: anyhow::Result<ExtendedHash> = (|| {
                if args[0].len() > 8 {
                    let h = u64::from_be(u64::from_str_radix(args[0], 16)?);
                    Ok(ExtendedHash::Hash64(TagHash64(h)))
                } else {
                    let h = u32::from_be(u32::from_str_radix(args[0], 16)?);
                    Ok(ExtendedHash::Hash32(TagHash(h)))
                }
            })();

            let tag = match tag_parsed {
                Ok(o) => o,
                Err(e) => {
                    error!("Failed to parse tag: {e}");
                    return;
                }
            };

            if let Some(mut viewers) = resources.get_mut::<ViewerWindows>() {
                let dcs = resources.get::<DcsShared>().unwrap();
                match TextureViewer::new(tag, &dcs) {
                    Ok(o) => {
                        info!("Successfully loaded texture {tag}");
                        viewers.0.push(Box::new(o));
                    }
                    Err(e) => {
                        error!("Failed to load texture {tag}: {e}");
                    }
                }
            }
        }
        "open.mat" | "open.material" => {}
        "clear_map" => {
            if let Some(mut maps) = resources.get_mut::<MapDataList>() {
                let current_map = maps.current_map;
                let scene = &mut maps.maps[current_map].2.scene;
                scene.clear();
            }
        }
        "spawn_entity_model" => {
            if let Some(mut maps) = resources.get_mut::<MapDataList>() {
                // TODO(cohae): Make some abstraction for this
                if args.len() != 1 {
                    error!("Missing tag argument, expected 32/64-bit tag");
                    return;
                }

                let tag_parsed: anyhow::Result<ExtendedHash> = (|| {
                    if args[0].len() > 8 {
                        let h = u64::from_be(u64::from_str_radix(args[0], 16)?);
                        Ok(ExtendedHash::Hash64(TagHash64(h)))
                    } else {
                        let h = u32::from_be(u32::from_str_radix(args[0], 16)?);
                        Ok(ExtendedHash::Hash32(TagHash(h)))
                    }
                })();

                let tag = match tag_parsed {
                    Ok(o) => o,
                    Err(e) => {
                        error!("Failed to parse tag: {e}");
                        return;
                    }
                };

                let current_map = maps.current_map;
                let scene = &mut maps.maps[current_map].2.scene;
                let camera = resources.get::<FpsCamera>().unwrap();

                let renderer = resources.get_mut::<RendererShared>().unwrap();
                let rb = renderer.read();
                println!("Spawning entity {tag}...");
                match load_entity_model(tag, &rb) {
                    Ok(er) => {
                        scene.spawn((
                            Transform {
                                translation: camera.position,
                                ..Default::default()
                            },
                            EntityModel(er),
                        ));
                    }
                    Err(e) => error!("Failed to load entitymodel {tag}: {e}"),
                }
            }
        }
        "disassemble_tfx" => {
            if args.is_empty() {
                error!("Missing bytes argument, expected hex bytestream");
                return;
            }

            let hex_stream = args.iter().join("").replace(' ', "");
            let data = match hex::decode(hex_stream) {
                Ok(o) => o,
                Err(e) => {
                    error!("Invalid hex data: {e}");
                    return;
                }
            };

            let opcodes = match TfxBytecodeOp::parse_all(&data, binrw::Endian::Little) {
                Ok(o) => o,
                Err(e) => {
                    error!("Failed to decode TFX bytecode: {e}");
                    return;
                }
            };

            info!("TFX Disassembly:");
            for (i, o) in opcodes.into_iter().enumerate() {
                info!(" {i}: {}", o.disassemble());
            }

            // 3C0100340003293401340212232200350334050E44043C01003406032934073408122322003509340B0E440D
        }
        _ => error!("Unknown command '{command}'"),
    }
}

fn load_entity_model(t: ExtendedHash, renderer: &Renderer) -> anyhow::Result<EntityRenderer> {
    let model: SEntityModel =
        package_manager().read_tag_struct(t.hash32().context("Couldnt lookup hash64")?)?;

    for m in &model.meshes {
        for p in &m.parts {
            if p.material.is_some() {
                renderer.render_data.data_mut().materials.insert(
                    p.material,
                    Technique::load(
                        renderer,
                        package_manager().read_tag_struct(p.material)?,
                        p.material,
                        true,
                    ),
                );
            }
        }
    }

    EntityRenderer::load(model, vec![], vec![], renderer)
}
