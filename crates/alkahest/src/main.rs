#![warn(rust_2018_idioms)]
#![deny(clippy::correctness, clippy::suspicious, clippy::complexity)]
#![allow(clippy::collapsible_else_if, clippy::missing_transmute_annotations)]

#[macro_use]
extern crate tracing;

use std::{fmt::Write, path::PathBuf, process::exit, str::FromStr, sync::Arc};

use alkahest_pm::PACKAGE_MANAGER;
use alkahest_renderer::util::image::Png;
use anyhow::Context;
use app::AlkahestApp;
use clap::Parser;
use destiny_pkg::{GameVersion, PackageManager, TagHash};
use mimalloc::MiMalloc;
use tracing::level_filters::LevelFilter;
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};
use util::consts;
use winit::event_loop::EventLoop;

use crate::gui::console::ConsoleLogLayer;

mod app;
mod config;
mod game_selector;
mod gui;
mod maplist;
mod resources {
    pub use alkahest_renderer::resources::*;
}
mod discord;
mod updater;
mod util;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, disable_version_flag(true))]
struct ApplicationArgs {
    /// Packages directory
    package_dir: Option<String>,

    // TODO(cohae): Reimplement
    // /// Package prefix to load maps from, ignores package argument.
    // /// For example: `throneworld`, `edz`
    // #[arg(short, long)]
    // package_name: Option<String>,
    /// Map hash to load. Ignores package_name argument
    #[arg(short, long, value_parser = parse_taghash)]
    map: Option<TagHash>,

    #[arg(short, long, value_parser = parse_taghash)]
    activity: Option<TagHash>,

    #[arg(long, alias = "na")]
    no_ambient: bool,

    #[arg(long)]
    low_res: bool,

    #[arg(long)]
    fullscreen: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    util::fix_windows_command_prompt();

    let mut panic_header = String::new();
    writeln!(&mut panic_header, "Alkahest v{}", consts::VERSION).unwrap();
    writeln!(&mut panic_header, "Built from commit {}", consts::GIT_HASH).unwrap();
    writeln!(&mut panic_header, "Built on {}", consts::BUILD_TIMESTAMP).unwrap();

    alkahest_panic_handler::install_hook(Some(panic_header));

    consts::print_banner();

    config::load();

    #[cfg(feature = "deadlock_detection")]
    {
        // only for #[cfg]
        use std::{thread, time::Duration};

        use parking_lot::deadlock;

        // Create a background thread which checks for deadlocks every 10s
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(10));
            let deadlocks = deadlock::check_deadlock();
            if deadlocks.is_empty() {
                continue;
            }

            println!("{} deadlocks detected", deadlocks.len());
            for (i, threads) in deadlocks.iter().enumerate() {
                println!("Deadlock #{}", i);
                for t in threads {
                    println!("Thread Id {:#?}", t.thread_id());
                    println!("{:#?}", t.backtrace());
                }
            }
        });
    } // only for #[cfg]

    let args = ApplicationArgs::parse();
    config::with_mut(|c| {
        c.window.fullscreen = args.fullscreen;
    });

    rayon::ThreadPoolBuilder::new()
        .thread_name(|i| format!("rayon-worker-{i}"))
        .num_threads(3)
        .build_global()
        .unwrap();

    // Remove the original log, if it exists
    std::fs::remove_file("./alkahest.log").ok();
    let file_appender = tracing_appender::rolling::never("./", "alkahest.log");

    LogTracer::init()?;
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(ConsoleLogLayer)
            .with(
                tracing_subscriber::fmt::layer()
                    .without_time()
                    .with_ansi(false)
                    .with_writer(file_appender),
            )
            .with(tracing_subscriber::fmt::layer().without_time())
            .with(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            ),
    )
    .expect("Failed to set up the tracing subscriber");

    let icon_data = Png::from_bytes(include_bytes!("../assets/icon.png"))?;
    let icon = winit::window::Icon::from_rgba(
        icon_data.data.to_vec(),
        icon_data.dimensions[0] as u32,
        icon_data.dimensions[1] as u32,
    )
    .unwrap();

    let mut event_loop = EventLoop::new()?;
    initialize_package_manager(&args, &mut event_loop, &icon)?;

    // extract_tfx_externs()?;

    tokio::spawn(discord::discord_client_loop());

    let mut app = AlkahestApp::new(event_loop, &icon, args);

    app.run()?;

    // cohae: Workaround for a weird freeze when trying to close alkahest normally, might have something to do with the discord client thread
    drop(app);
    exit(0);
}

fn initialize_package_manager(
    args: &ApplicationArgs,
    event_loop: &mut EventLoop<()>,
    icon: &winit::window::Icon,
) -> anyhow::Result<()> {
    let package_dir = if let Some(p) = &args.package_dir {
        if p.ends_with(".pkg") {
            warn!(
                "Please specify the directory containing the packages, not the package itself! \
                 Support for this will be removed in the future!"
            );
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
            &game_selector::select_game_installation(event_loop, icon)
                .context("No game installation selected")?,
        )
        .unwrap();

        path.join("packages")
    };

    if !package_dir.exists() {
        config::with_mut(|c| c.packages_directory = None);
        config::persist();

        panic!(
            "The specified package directory does not exist! ({})\nRelaunch alkahest with a valid \
             package directory.",
            package_dir.display()
        );
    }

    let pm = info_span!("Initializing package manager").in_scope(|| {
        PackageManager::new(package_dir, GameVersion::Destiny2TheFinalShape, None).unwrap()
    });

    config::with_mut(|c| c.packages_directory = Some(pm.package_dir.to_string_lossy().to_string()));
    config::persist();

    *PACKAGE_MANAGER.write() = Some(Arc::new(pm));

    Ok(())
}

pub fn parse_taghash(s: &str) -> Result<TagHash, String> {
    const HEX_PREFIX: &str = "0x";
    const HEX_PREFIX_UPPER: &str = "0X";
    const HEX_PREFIX_LEN: usize = HEX_PREFIX.len();

    let result = if s.starts_with(HEX_PREFIX) || s.starts_with(HEX_PREFIX_UPPER) {
        u32::from_str_radix(&s[HEX_PREFIX_LEN..], 16)
    } else {
        u32::from_str_radix(s, 16)
    }
    .map(|v| TagHash(u32::from_be(v)));

    result.map_err(|e| e.to_string())
}

// fn extract_tfx_externs() -> anyhow::Result<()> {
//     use tiger_parse::TigerReadable;
//     #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
//     pub enum ExternFieldType {
//         Float,
//         Vec4,
//         Mat4,
//         U32,
//         Texture,
//         Uav,
//     }

//     let mut fields: FxHashSet<(TfxExtern, ExternFieldType, usize)> = Default::default();

//     for (t, _) in package_manager()
//         .get_all_by_reference(SScope::ID.unwrap())
//         .into_iter()
//     {
//         let scope: SScope = package_manager().read_tag_struct(t)?;
//         for s in scope.iter_stages() {
//             if let Ok(opcodes) =
//                 TfxBytecodeOp::parse_all(&s.constants.bytecode, binrw::Endian::Little)
//             {
//                 for op in opcodes {
//                     match op {
//                         TfxBytecodeOp::PushExternInputFloat { extern_, offset } => {
//                             fields.insert((extern_, ExternFieldType::Float, offset as usize * 4));
//                         }
//                         TfxBytecodeOp::PushExternInputVec4 { extern_, offset } => {
//                             fields.insert((extern_, ExternFieldType::Vec4, offset as usize * 16));
//                         }
//                         TfxBytecodeOp::PushExternInputMat4 { extern_, offset } => {
//                             fields.insert((extern_, ExternFieldType::Mat4, offset as usize * 16));
//                         }
//                         TfxBytecodeOp::PushExternInputTextureView { extern_, offset } => {
//                             fields.insert((extern_, ExternFieldType::Texture, offset as usize * 8));
//                         }
//                         TfxBytecodeOp::PushExternInputU32 { extern_, offset } => {
//                             fields.insert((extern_, ExternFieldType::U32, offset as usize * 4));
//                         }
//                         TfxBytecodeOp::PushExternInputUav { extern_, offset } => {
//                             fields.insert((extern_, ExternFieldType::Uav, offset as usize * 8));
//                         }
//                         _ => {}
//                     }
//                 }
//             }
//         }
//     }

//     for (t, _) in package_manager()
//         .get_all_by_reference(STechnique::ID.unwrap())
//         .into_iter()
//     {
//         let Ok(technique): anyhow::Result<STechnique> = package_manager().read_tag_struct(t) else {
//             continue;
//         };
//         for (_, s) in technique.all_shaders() {
//             if let Ok(opcodes) =
//                 TfxBytecodeOp::parse_all(&s.constants.bytecode, binrw::Endian::Little)
//             {
//                 for op in opcodes {
//                     match op {
//                         TfxBytecodeOp::PushExternInputFloat { extern_, offset } => {
//                             fields.insert((extern_, ExternFieldType::Float, offset as usize * 4));
//                         }
//                         TfxBytecodeOp::PushExternInputVec4 { extern_, offset } => {
//                             fields.insert((extern_, ExternFieldType::Vec4, offset as usize * 16));
//                         }
//                         TfxBytecodeOp::PushExternInputMat4 { extern_, offset } => {
//                             fields.insert((extern_, ExternFieldType::Mat4, offset as usize * 16));
//                         }
//                         TfxBytecodeOp::PushExternInputTextureView { extern_, offset } => {
//                             fields.insert((extern_, ExternFieldType::Texture, offset as usize * 8));
//                         }
//                         TfxBytecodeOp::PushExternInputU32 { extern_, offset } => {
//                             fields.insert((extern_, ExternFieldType::U32, offset as usize * 4));
//                         }
//                         TfxBytecodeOp::PushExternInputUav { extern_, offset } => {
//                             fields.insert((extern_, ExternFieldType::Uav, offset as usize * 8));
//                         }
//                         _ => {}
//                     }
//                 }
//             }
//         }
//     }

//     // println!("Fields: {fields:#?}");

//     for ext in TfxExtern::iter() {
//         let mut sfields = fields
//             .iter()
//             .filter(|(e, _, _)| *e == ext)
//             .map(|(_, a, b)| (*a, *b))
//             .collect_vec();

//         sfields.sort_by_key(|(_, offset)| *offset);

//         if sfields.is_empty() {
//             continue;
//         }

//         println!("struct {ext:?} {{");

//         for (ty, offset) in sfields {
//             let ty_str = match ty {
//                 ExternFieldType::Float => "f32",
//                 ExternFieldType::Vec4 => "Vec4",
//                 ExternFieldType::Mat4 => "Mat4",
//                 ExternFieldType::U32 => "u32",
//                 ExternFieldType::Texture => "TextureView",
//                 ExternFieldType::Uav => "UnorderedAccessView",
//             };

//             println!("\tpub unk{offset:02x}: {ty_str},");
//         }

//         println!("}}\n");
//     }

//     Ok(())
// }
