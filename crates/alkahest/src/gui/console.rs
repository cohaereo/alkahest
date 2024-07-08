use std::{
    fmt::Debug,
    io::{Cursor, Seek, SeekFrom},
    sync::Arc,
};

use alkahest_data::{
    entity::{SDynamicModel, SEntity, Unk808072c5},
    technique::STechnique,
    tfx::{TfxFeatureRenderer, TfxRenderStage},
    WideHash,
};
use alkahest_pm::package_manager;
use alkahest_renderer::{
    camera::Camera,
    ecs::{
        common::{Hidden, Icon, Label, Mutable},
        render::{dynamic_geometry::DynamicModelComponent, static_geometry::StaticModelSingle},
        tags::{EntityTag, Tags},
        transform::{OriginalTransform, Transform},
        utility::{Route, RouteNode},
    },
    icons::ICON_CUBE,
    renderer::{Renderer, RendererShared},
    resources::Resources,
    tfx::bytecode::{decompiler::TfxBytecodeDecompiler, opcodes::TfxBytecodeOp},
};
use anyhow::Context;
use binrw::BinReaderExt;
use destiny_pkg::{TagHash, TagHash64};
use egui::{Color32, RichText, TextStyle};
use glam::{Vec2, Vec3};
use hecs::DynamicBundle;
use itertools::Itertools;
use lazy_static::lazy_static;
use ringbuffer::{AllocRingBuffer, RingBuffer};
use rustc_hash::FxHashSet;
use tiger_parse::{Endian, PackageManagerExt, TigerReadable};
use tracing::{
    field::{Field, Visit},
    Event, Level, Subscriber, Value,
};
use tracing_subscriber::Layer;
use winit::window::Window;

use crate::{
    gui::{
        commands::load_pkg_entities,
        context::{GuiCtx, GuiView, ViewResult},
    },
    maplist::MapList,
    util::action::{ActionList, ActivitySwapAction, SpawnRouteAction},
};

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

pub struct ConsolePanel {
    pub command_buffer: String,
    pub autoscroll: bool,
    pub open: bool,
}

impl Default for ConsolePanel {
    fn default() -> Self {
        Self {
            command_buffer: "".to_string(),
            autoscroll: true,
            open: false,
        }
    }
}

impl GuiView for ConsolePanel {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        resources: &Resources,
        _gui: &GuiCtx<'_>,
    ) -> Option<ViewResult> {
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
                                let event = &c[row];
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

        None
    }
}

fn execute_command(command: &str, args: &[&str], resources: &Resources) {
    match command.to_lowercase().as_str() {
        "goto" => {
            if args.len() < 3 {
                error!("Too few arguments, expected 3/5, got {}", args.len());
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
                    let mut camera = resources.get_mut::<Camera>();
                    camera.set_position(new_pos);
                    info!("Teleported to {} {} {}", new_pos.x, new_pos.y, new_pos.z);
                }
                Err(e) => {
                    error!("Invalid coordinates: {e}");
                }
            }

            if args.len() >= 5 {
                let parsed_ang: anyhow::Result<Vec2> = (|| {
                    let x = str::parse(args[3])?;
                    let y = str::parse(args[4])?;

                    Ok(Vec2::new(x, y))
                })();

                match parsed_ang {
                    Ok(new_ang) => {
                        let mut camera = resources.get_mut::<Camera>();
                        camera.set_orientation(new_ang);
                    }
                    Err(e) => {
                        error!("Invalid angles: {e}");
                    }
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

            let mut camera = resources.get_mut::<Camera>();
            camera.set_position(new_pos);
            info!("Teleported to {} {} {}", new_pos.x, new_pos.y, new_pos.z);
        }
        // "open.tex" | "open.texture" => {
        //     if args.len() != 1 {
        //         error!("Missing tag argument, expected 32/64-bit tag");
        //         return;
        //     }
        //
        //     let tag = match parse_extended_hash(args[0]) {
        //         Ok(o) => o,
        //         Err(e) => {
        //             error!("Failed to parse tag: {e}");
        //             return;
        //         }
        //     };
        //
        //     if let Some(mut viewers) = resources.get_mut::<ViewerWindows>() {
        //         let dcs = resources.get::<DcsShared>().unwrap();
        //         match TextureViewer::new(tag, dcs.clone(), gui) {
        //             Ok(o) => {
        //                 info!("Successfully loaded texture {tag}");
        //                 // TODO(cohae): Focus window if already open
        //                 viewers
        //                     .0
        //                     .entry(tag.to_string())
        //                     .or_insert_with(|| Box::new(o));
        //             }
        //             Err(e) => {
        //                 error!("Failed to load texture {tag}: {e}");
        //             }
        //         }
        //     }
        // }
        // "open.tech" | "open.technique" | "open.mat" | "open.material" => {
        //     if args.len() != 1 {
        //         error!("Missing tag argument, expected 32/64-bit tag");
        //         return;
        //     }
        //
        //     let tag = match parse_extended_hash(args[0]) {
        //         Ok(o) => o,
        //         Err(e) => {
        //             error!("Failed to parse tag: {e}");
        //             return;
        //         }
        //     };
        //
        //     if let Some(mut viewers) = resources.get_mut::<ViewerWindows>() {
        //         let dcs = resources.get::<DcsShared>().unwrap();
        //         match TechniqueViewer::new(tag, dcs.clone(), gui) {
        //             Ok(o) => {
        //                 info!("Successfully loaded material {tag}");
        //                 // TODO(cohae): Focus window if already open
        //                 viewers
        //                     .0
        //                     .entry(tag.to_string())
        //                     .or_insert_with(|| Box::new(o));
        //             }
        //             Err(e) => {
        //                 error!("Failed to load material {tag}: {e}");
        //             }
        //         }
        //     }
        // }
        "clear_map" => {
            let mut maps = resources.get_mut::<MapList>();
            if let Some(map) = maps.current_map_mut() {
                map.scene.clear();
            }
        }
        "sem" | "spawn_entity_model" => {
            let mut maps = resources.get_mut::<MapList>();
            if args.len() != 1 {
                error!("Missing tag argument, expected 32/64-bit tag");
                return;
            }

            let tag = match parse_extended_hash(args[0]) {
                Ok(o) => o,
                Err(e) => {
                    error!("Failed to parse tag: {e}");
                    return;
                }
            };

            let Some(scene) = maps.current_map_mut().map(|m| &mut m.scene) else {
                return;
            };

            let camera = resources.get::<Camera>();

            let renderer = resources.get_mut::<RendererShared>();
            println!("Spawning entity {tag}...");
            let transform = Transform {
                translation: camera.position(),
                ..Default::default()
            };
            match load_entity_model(tag, transform, &renderer) {
                Ok(er) => {
                    scene.spawn(er);
                    info!("Entity model spawned");
                }
                Err(e) => error!("Failed to load entitymodel {tag}: {e}"),
            }
        }
        "se" | "spawn_entity" => {
            let mut maps = resources.get_mut::<MapList>();
            if args.len() != 1 {
                error!("Missing tag argument, expected 32/64-bit tag");
                return;
            }

            let tag = match parse_extended_hash(args[0]) {
                Ok(o) => o,
                Err(e) => {
                    error!("Failed to parse tag: {e}");
                    return;
                }
            };

            let Some(scene) = maps.current_map_mut().map(|m| &mut m.scene) else {
                return;
            };

            let camera = resources.get::<Camera>();

            let renderer = resources.get_mut::<RendererShared>();
            println!("Spawning entity {tag}...");
            let transform = Transform {
                translation: camera.position(),
                ..Default::default()
            };
            match load_entity(tag, transform, &renderer) {
                Ok(er) => {
                    scene.spawn(er);
                    info!("Entity spawned");
                }
                Err(e) => error!("Failed to load entity {tag}: {e:?}"),
            }
        }
        "ss" | "spawn_static" => {
            let mut maps = resources.get_mut::<MapList>();
            if args.len() != 1 {
                error!("Missing tag argument, expected 32/64-bit tag");
                return;
            }

            let tag = match parse_extended_hash(args[0]) {
                Ok(o) => o,
                Err(e) => {
                    error!("Failed to parse tag: {e}");
                    return;
                }
            };

            let Some(scene) = maps.current_map_mut().map(|m| &mut m.scene) else {
                return;
            };

            let camera = resources.get::<Camera>();

            let renderer = resources.get_mut::<RendererShared>();
            let mut rdata = renderer.data.lock();

            match StaticModelSingle::load(
                renderer.gpu.clone(),
                &mut rdata.asset_manager,
                tag.hash32(),
            ) {
                Ok(er) => {
                    let transform = Transform {
                        translation: camera.position(),
                        ..Default::default()
                    };
                    scene.spawn((
                        er,
                        Icon::Unicode(ICON_CUBE),
                        Label::from(format!("Static Model {}", tag.hash32())),
                        transform,
                        TfxFeatureRenderer::StaticObjects,
                        Mutable,
                        Tags::from_iter([EntityTag::User]),
                    ));
                    info!("Static model spawned");
                }
                Err(e) => error!("Failed to load static model {tag}: {e:?}"),
            }
        }
        "distfx" | "disassemble_tfx" => {
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
                info!(" {i}: {}", o.disassemble(None));
            }
        }
        "distfxt" | "disassemble_tfx_technique" => {
            // TODO(cohae): Make some abstraction for this
            if args.len() != 1 {
                error!("Missing tag argument, expected 32-bit tag");
                return;
            }

            let tag = match parse_extended_hash(args[0]) {
                Ok(o) => o.hash32(),
                Err(e) => {
                    error!("Failed to parse tag: {e}");
                    return;
                }
            };

            let technique: STechnique = match package_manager().read_tag_struct(tag) {
                Ok(o) => o,
                Err(e) => {
                    error!("Failed to read technique tag: {e}");
                    return;
                }
            };

            for (stage, shader) in technique.all_valid_shaders() {
                let opcodes = match TfxBytecodeOp::parse_all(
                    &shader.constants.bytecode,
                    binrw::Endian::Little,
                ) {
                    Ok(o) => o,
                    Err(e) => {
                        error!("Failed to decode TFX bytecode: {e}");
                        return;
                    }
                };

                let constants = if shader.constants.bytecode_constants.is_empty() {
                    &[]
                } else {
                    bytemuck::cast_slice(&shader.constants.bytecode_constants)
                };

                println!();
                info!("TFX Disassembly ({stage:?}):");
                for (i, o) in opcodes.iter().enumerate() {
                    info!("  {i}: {}", o.disassemble(Some(constants)));
                }

                if let Ok(e) =
                    TfxBytecodeDecompiler::decompile(&opcodes, constants).map(|e| e.pretty_print())
                {
                    info!("Decompiled bytecode:");
                    for l in e.lines() {
                        info!("  {l}");
                    }
                }
            }
        }
        "reset_all_to_original_pos" => {
            let maps = resources.get::<MapList>();
            if let Some(map) = maps.current_map() {
                for (_, (t, ot)) in map
                    .scene
                    .query::<(&mut Transform, &OriginalTransform)>()
                    .iter()
                {
                    *t = ot.0;
                }
            }
        }
        "unhide_all" | "show_all" => {
            let mut maps = resources.get_mut::<MapList>();
            if let Some(map) = maps.current_map_mut() {
                let entities = map
                    .scene
                    .query::<&Hidden>()
                    .iter()
                    .map(|(e, _)| e)
                    .collect_vec();
                for e in entities {
                    map.scene.remove_one::<Hidden>(e).ok();
                }
            }
        }
        "clear_maplist" => {
            let mut maps = resources.get_mut::<MapList>();
            maps.set_maps(resources, &[]);
        }
        "route" => {
            let mut route = Route::default();
            let mut i: usize = 0;
            if args[i].to_lowercase().as_str() == "hash" {
                i += 1;
                if args.len() < i {
                    error!("missing hash value");
                    return;
                }
                let parsed_hash: anyhow::Result<TagHash> = (|| {
                    let hash = str::parse(args[i])?;
                    i += 1;
                    Ok(TagHash(hash))
                })();
                match parsed_hash {
                    Ok(new_hash) => {
                        route.activity_hash = Some(new_hash);
                    }
                    Err(e) => {
                        error!("Invalid hash: {e}");
                        return;
                    }
                }
            }
            while i < args.len() {
                let mut node = RouteNode::default();
                match args[i].to_lowercase().as_str() {
                    "node" => 'node: {
                        i += 1;
                        if args.len() < 3 + i {
                            error!(
                                "Too few arguments for node coordinates. Expected 3, got {}",
                                args.len() - i
                            );
                            return;
                        }
                        let parsed_pos: anyhow::Result<Vec3> = (|| {
                            let x = str::parse(args[i])?;
                            let y = str::parse(args[i + 1])?;
                            let z = str::parse(args[i + 2])?;
                            i += 3;

                            Ok(Vec3::new(x, y, z))
                        })();

                        match parsed_pos {
                            Ok(new_pos) => {
                                node.pos = new_pos;
                            }
                            Err(e) => {
                                error!("Invalid coordinates: {e}");
                                return;
                            }
                        }

                        if args.len() <= i {
                            break 'node;
                        }

                        if args[i] == "tp" {
                            i += 1;
                            node.is_teleport = true;
                        }

                        if args[i] == "hash" {
                            i += 1;
                            if args.len() < i {
                                error!("missing hash value");
                                return;
                            }
                            let parsed_hash: anyhow::Result<TagHash> = (|| {
                                let hash = str::parse(args[i])?;
                                i += 1;
                                Ok(TagHash(hash))
                            })(
                            );
                            match parsed_hash {
                                Ok(new_hash) => {
                                    node.map_hash = Some(new_hash);
                                }
                                Err(e) => {
                                    error!("Invalid hash: {e}");
                                    return;
                                }
                            }
                        }

                        if args.len() <= i {
                            break 'node;
                        }

                        if args[i] == "label" {
                            i += 1;
                            if args.len() <= i {
                                error!("label requires a string");
                                return;
                            }
                            node.label = Some(args[i].replace(r"\s", " ").replace(r"\\", r"\"));
                            i += 1;
                        }
                    }
                    _ => {
                        error!(
                            r"Expected node x y z [hash HASH] [label string\swith\sno\sspace\\single\sslash]"
                        );
                        return;
                    }
                }
                route.path.push(node);
            }

            let mut action_list = resources.get_mut::<ActionList>();
            if let Some(hash) = route.activity_hash {
                action_list.add_action(ActivitySwapAction::new(hash));
            };
            action_list.add_action(SpawnRouteAction::new(route));
        }
        "inspect.mat" => {
            // TODO(cohae): Make some abstraction for this
            if args.len() != 1 {
                error!("Missing tag argument, expected 32-bit tag");
                return;
            }

            let tag = match parse_extended_hash(args[0]) {
                Ok(o) => o.hash32(),
                Err(e) => {
                    error!("Failed to parse tag: {e}");
                    return;
                }
            };

            let technique: STechnique = match package_manager().read_tag_struct(tag) {
                Ok(o) => o,
                Err(e) => {
                    error!("Failed to read technique tag: {e}");
                    return;
                }
            };

            println!("Technique {tag}:");
            println!("\tUsed scopes:");
            for scope in technique.used_scopes.iter() {
                println!("\t\t- {scope:?}");
            }
            println!("\tCompatible scopes:");
            for scope in technique.compatible_scopes.iter() {
                println!("\t\t- {scope:?}");
            }
        }
        "inspect.model" => {
            // TODO(cohae): Make some abstraction for this
            if args.len() != 1 {
                error!("Missing tag argument, expected 32-bit tag");
                return;
            }

            let tag = match parse_extended_hash(args[0]) {
                Ok(o) => o.hash32(),
                Err(e) => {
                    error!("Failed to parse tag: {e}");
                    return;
                }
            };

            let model: SDynamicModel = match package_manager().read_tag_struct(tag) {
                Ok(o) => o,
                Err(e) => {
                    error!("Failed to read dynamic model tag: {e}");
                    return;
                }
            };

            println!("Model {tag}:");
            for (i, m) in model.meshes.iter().enumerate() {
                println!("\tMesh {i}:");
                println!("\t\t{} parts", m.parts.len());
                println!("\t\tSupported renderstages:");
                for stage in TfxRenderStage::VARIANTS {
                    let range = m.get_range_for_stage(stage);
                    let unique_shaders: FxHashSet<TagHash> =
                        m.parts[range.clone()].iter().map(|p| p.technique).collect();
                    let unique_flags: FxHashSet<u32> =
                        m.parts[range.clone()].iter().map(|p| p.flags).collect();

                    if !range.is_empty() {
                        println!(
                            "\t\t\t- {stage:?}: {} parts (techniques: [{}] flags: [{}])",
                            range.len(),
                            unique_shaders.iter().map(|s| format!("{s}")).join(", "),
                            unique_flags.iter().map(|v| format!("0x{v:X}")).join(", ")
                        );
                    }
                }
            }
        }
        "load_entities_pkg" => {
            // TODO(cohae): Make some abstraction for this
            if args.len() != 1 {
                error!("Missing package name argument");
                return;
            }

            let mut maps = resources.get_mut::<MapList>();

            let Some(scene) = maps.current_map_mut().map(|m| &mut m.scene) else {
                return;
            };

            let renderer = resources.get_mut::<RendererShared>();

            if let Err(e) = load_pkg_entities(&args[0], renderer.clone(), scene) {
                error!("Failed to load entities from package {}: {e}", args[0]);
            }
        }
        _ => error!("Unknown command '{command}'"),
    }
}

pub fn load_entity_model(
    t: WideHash,
    transform: Transform,
    renderer: &Renderer,
) -> anyhow::Result<impl DynamicBundle> {
    Ok((
        Icon::Unicode(ICON_CUBE),
        Label::from("Entity Model"),
        transform,
        DynamicModelComponent::load(
            renderer,
            &transform,
            t.into(),
            vec![],
            vec![],
            TfxFeatureRenderer::DynamicObjects,
        )?,
        TfxFeatureRenderer::DynamicObjects,
        Mutable,
        Tags::from_iter([EntityTag::User]),
    ))
}

pub fn load_entity(
    entity_hash: WideHash,
    transform: Transform,
    renderer: &Renderer,
) -> anyhow::Result<impl DynamicBundle> {
    let header = package_manager()
        .read_tag_struct::<SEntity>(entity_hash)
        .context("Failed to read SEntity")?;
    for e in &header.entity_resources {
        match e.unk0.unk10.resource_type {
            0x80806d8a => {
                let mut cur = Cursor::new(package_manager().read_tag(e.unk0.taghash())?);
                cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x224))?;
                let model_hash: TagHash = TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x3c0))?;
                let entity_material_map: Vec<Unk808072c5> =
                    TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                cur.seek(SeekFrom::Start(e.unk0.unk18.offset + 0x400))?;
                let materials: Vec<TagHash> =
                    TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                return Ok((
                    Icon::Unicode(ICON_CUBE),
                    Label::from("Entity"),
                    transform,
                    DynamicModelComponent::load(
                        renderer,
                        &transform,
                        model_hash,
                        entity_material_map,
                        materials,
                        TfxFeatureRenderer::DynamicObjects,
                    )?,
                    TfxFeatureRenderer::DynamicObjects,
                    Mutable,
                    Tags::from_iter([EntityTag::User]),
                ));
            }
            u => {
                debug!(
                    "\t- Unknown entity resource type {:08X}/{:08X} (table {})",
                    u.to_be(),
                    e.unk0.unk10.resource_type.to_be(),
                    e.unk0.taghash()
                )
            }
        }
    }

    Err(anyhow::anyhow!("No entitymodel found in entity"))
}

fn parse_extended_hash(s: &str) -> anyhow::Result<WideHash> {
    let tag_parsed: anyhow::Result<WideHash> = (|| {
        if s.len() > 8 {
            let h = u64::from_be(u64::from_str_radix(s, 16)?);
            Ok(WideHash::Hash64(TagHash64(h)))
        } else {
            let h = u32::from_be(u32::from_str_radix(s, 16)?);
            Ok(WideHash::Hash32(TagHash(h)))
        }
    })();

    let tag = match tag_parsed {
        Ok(o) => o,
        Err(e) => {
            anyhow::bail!("Failed to parse tag: {e}");
        }
    };

    Ok(tag)
}
