#![warn(rust_2018_idioms)]
#![deny(clippy::correctness, clippy::suspicious, clippy::complexity)]
#![allow(clippy::collapsible_else_if)]

#[macro_use]
extern crate tracing;

use std::{fmt::Write, path::PathBuf, str::FromStr, sync::Arc};

use alkahest_pm::PACKAGE_MANAGER;
use alkahest_renderer::util::image::Png;
use anyhow::Context;
use app::AlkahestApp;
use clap::Parser;
use destiny_pkg::{PackageManager, PackageVersion, TagHash};
use mimalloc::MiMalloc;
use tracing::level_filters::LevelFilter;
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};
use util::consts;
use winit::event_loop::EventLoop;

use crate::gui::console::ConsoleLogLayer;

mod app;
mod config;
mod data;
mod game_selector;
mod gui;
mod maplist;
mod resources {
    pub use alkahest_renderer::resources::*;
}
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

    LogTracer::init()?;
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(ConsoleLogLayer)
            .with(tracing_subscriber::fmt::layer())
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

    let mut app = AlkahestApp::new(event_loop, &icon, args);

    app.run()
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
        PackageManager::new(package_dir, PackageVersion::Destiny2TheFinalShape).unwrap()
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
