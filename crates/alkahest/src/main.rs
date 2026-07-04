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
use tiger_pkg::{register_pkg_key, GameVersion, PackageManager, TagHash};
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
mod paths;
mod util;

// #[cfg(feature = "profiler")]
// #[global_allocator]
// static GLOBAL: profiling::tracy_client::ProfiledAllocator<std::alloc::System> =
//     profiling::tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

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
        PackageManager::new(
            package_dir,
            GameVersion::Destiny(tiger_pkg::DestinyVersion::Destiny2Renegades),
            None,
        )
        .unwrap()
    });

    config::with_mut(|c| c.packages_directory = Some(pm.package_dir.to_string_lossy().to_string()));
    config::persist();

    *PACKAGE_MANAGER.write() = Some(Arc::new(pm));
    register_all_keys();

    Ok(())
}

#[rustfmt::skip]
fn register_all_keys() {
    register_pkg_key(0xE61DB5D0909B17A1, [0x66, 0xC2, 0x53, 0x93, 0x21, 0x06, 0x36, 0xBA, 0x25, 0xF9, 0xE6, 0xE3, 0xCA, 0xBF, 0xF1, 0x52], [0xD8, 0x21, 0x91, 0x16, 0xE9, 0xB0, 0xDC, 0xCC, 0xA0, 0x9D, 0x49, 0x5F]); // w64_sr_warp_raid_assets_redacted
    register_pkg_key(0xDEADD3A99F8EB5D3, [0x66, 0xC2, 0x53, 0x93, 0x21, 0x06, 0x36, 0xBA, 0x25, 0xF9, 0xE6, 0xE3, 0xCA, 0xBF, 0xF1, 0x52], [0xD8, 0x21, 0x91, 0x16, 0xE9, 0xB0, 0xDC, 0xCC, 0xA0, 0x9D, 0x49, 0x5F]); // w64_warp_raid_redacted
    register_pkg_key(0x130142B549C8A286, [0xC0, 0xF5, 0x27, 0x35, 0x87, 0xD2, 0x14, 0xA6, 0x10, 0x47, 0x30, 0x30, 0x8D, 0xB4, 0xED, 0xF8], [0x35, 0x0D, 0x38, 0x86, 0x58, 0xBA, 0x44, 0x38, 0xB6, 0x19, 0xC8, 0xBD]); // w64_crucible_mall_redacted
    register_pkg_key(0x2B80907FC71F2A53, [0x51, 0xE5, 0x4F, 0xA6, 0xEE, 0x0E, 0x0D, 0x13, 0x23, 0xEA, 0xD8, 0xA3, 0x6E, 0xF8, 0x33, 0x5D], [0x60, 0x49, 0xCF, 0xAC, 0x73, 0x59, 0xA9, 0x5F, 0x93, 0xE3, 0xC3, 0xEB]); // w64_crucible_ice_redacted
    register_pkg_key(0x5B34A52A5D2AC948, [0xCA, 0xA7, 0x76, 0x59, 0x4E, 0x6F, 0x1D, 0x79, 0x49, 0x8D, 0x79, 0x6D, 0x14, 0x1B, 0x34, 0x3B], [0x6F, 0xD6, 0x50, 0x3B, 0x2A, 0xA8, 0x50, 0x63, 0x7B, 0x5C, 0x93, 0x8D]); // w64_crucible_root_redacted
    register_pkg_key(0x70AFE34BFB31A9E3, [0x35, 0x4A, 0x90, 0x35, 0x24, 0xA2, 0xAE, 0xC3, 0x2D, 0xD7, 0x45, 0x36, 0xC0, 0xAC, 0x08, 0x86], [0x21, 0x12, 0xF6, 0x7C, 0x16, 0xDD, 0x3F, 0xA2, 0x1A, 0x7A, 0x45, 0xBC]); // w64_sr_s22_redacted
    register_pkg_key(0xECF381534E93C17A, [0xB1, 0x17, 0x6C, 0x5A, 0x7B, 0xEA, 0x3A, 0x07, 0x6F, 0x8F, 0xB0, 0x5F, 0x90, 0x3C, 0x70, 0x59], [0xA0, 0x78, 0xF1, 0x3F, 0x36, 0x39, 0x09, 0x69, 0xD8, 0x3D, 0xDC, 0x31]); // w64_sr_s22_sc_redacted
    register_pkg_key(0x81C909CE22A85E8C, [0x06, 0x05, 0xCE, 0x48, 0x1D, 0xBE, 0x3D, 0x50, 0x42, 0xEB, 0xF6, 0x09, 0x1A, 0xA0, 0x5F, 0xF6], [0x11, 0x59, 0xD0, 0x53, 0x21, 0x52, 0x10, 0x74, 0x8A, 0x10, 0xF0, 0x3B]); // w64_sr_v800_act1_redacted
    register_pkg_key(0x472BE1A9A473E5AD, [0xB4, 0x66, 0xB7, 0x1E, 0x75, 0xAD, 0x3A, 0xDC, 0x3A, 0x05, 0x80, 0xBE, 0x1C, 0x6D, 0x81, 0x5D], [0x4A, 0x00, 0x27, 0x9A, 0xB3, 0x51, 0x8E, 0xD8, 0x4B, 0xEB, 0x59, 0xA3]); // w64_sr_v800_act2_redacted
    register_pkg_key(0x90353B5860A1EBB1, [0xC6, 0x72, 0x48, 0xA1, 0x78, 0xFA, 0x86, 0x11, 0x48, 0x31, 0xE4, 0x2B, 0x82, 0x8C, 0xFC, 0xA2], [0xF2, 0xB8, 0x2C, 0xEB, 0xEA, 0xFE, 0xDA, 0x2B, 0xA4, 0x8E, 0x2A, 0x7B]); // w64_v810_dungeon_map_redacted
    register_pkg_key(0xD34B426BF6DF81E2, [0xC6, 0x72, 0x48, 0xA1, 0x78, 0xFA, 0x86, 0x11, 0x48, 0x31, 0xE4, 0x2B, 0x82, 0x8C, 0xFC, 0xA2], [0xF2, 0xB8, 0x2C, 0xEB, 0xEA, 0xFE, 0xDA, 0x2B, 0xA4, 0x8E, 0x2A, 0x7B]); // w64_sr_v810_dungeon_redacted
    register_pkg_key(0xDF597BCAFAC50D42, [0x53, 0x1F, 0x5F, 0xE9, 0x43, 0xD7, 0x71, 0xA4, 0x17, 0x89, 0x76, 0x2D, 0x22, 0x99, 0x65, 0xCE], [0xA8, 0x2C, 0x18, 0x0D, 0x7C, 0x29, 0xD1, 0xA4, 0x2F, 0x25, 0x3E, 0x8E]); // w64_sr_v810_act2_redacted
    register_pkg_key(0x51280AE890B282AD, [0x0F, 0x60, 0xE7, 0xD4, 0xFD, 0x32, 0xF2, 0x16, 0x51, 0x1A, 0xFF, 0xCD, 0x90, 0x43, 0x3F, 0xB4], [0x6E, 0x66, 0x35, 0x69, 0x9A, 0x59, 0x40, 0x55, 0xC0, 0x7F, 0xB4, 0xAF]); // w64_sr_v810_act3_redacted
    register_pkg_key(0x548A073EBF282441, [0xD0, 0x64, 0x25, 0xD0, 0xC0, 0xCB, 0xC8, 0x8F, 0x9C, 0x37, 0xAE, 0xA2, 0x46, 0x11, 0x7A, 0xDF], [0x25, 0xDB, 0xEC, 0x12, 0x47, 0x64, 0xB8, 0x03, 0x31, 0x9E, 0xEF, 0x6F]); // w64_sr_v820_act2_redacted_a
    register_pkg_key(0x548A043EBF281F28, [0x7A, 0x57, 0xBA, 0xCE, 0xDA, 0xD6, 0x71, 0x70, 0x8D, 0xB6, 0x1B, 0xBB, 0xE7, 0x59, 0xFD, 0x86], [0xBF, 0xF5, 0xF1, 0x80, 0x41, 0x69, 0xB6, 0x17, 0x21, 0xD4, 0x42, 0x8F]); // w64_sr_v820_act2_redacted_b
    register_pkg_key(0x532BDBB3EAE790EC, [0xA8, 0xC3, 0x23, 0xB6, 0xB1, 0x8B, 0x0E, 0xF8, 0x1A, 0xAB, 0x54, 0x45, 0x40, 0x96, 0x8B, 0x30], [0xC5, 0x77, 0x50, 0x60, 0x73, 0xFB, 0x64, 0x78, 0xBA, 0x74, 0x80, 0xFB]); // w64_sr_v820_act3_redacted
    register_pkg_key(0x8E28CB894CD7D6CF, [0x18, 0xB3, 0xB6, 0xDE, 0xBB, 0xBE, 0x28, 0xAE, 0x7A, 0x65, 0xD7, 0x30, 0xFD, 0xAD, 0x28, 0xC0], [0xFC, 0x73, 0xE1, 0xD8, 0x85, 0x4B, 0x3D, 0xE4, 0xE3, 0x07, 0x77, 0x79]); // w64_sr_v820_dungeon_redacted
    register_pkg_key(0xCEA3E83FFFB9BF29, [0x18, 0xB3, 0xB6, 0xDE, 0xBB, 0xBE, 0x28, 0xAE, 0x7A, 0x65, 0xD7, 0x30, 0xFD, 0xAD, 0x28, 0xC0], [0xFC, 0x73, 0xE1, 0xD8, 0x85, 0x4B, 0x3D, 0xE4, 0xE3, 0x07, 0x77, 0x79]); // w64_dungeon_delver_redacted
    register_pkg_key(0xDC338B7D324AEC2F, [0xDB, 0x22, 0x15, 0x95, 0x0B, 0xCA, 0xBC, 0x8A, 0x5D, 0x94, 0xD3, 0x05, 0x98, 0xAE, 0x1B, 0xDA], [0xF8, 0x85, 0x45, 0x39, 0x8E, 0xDC, 0x58, 0x4E, 0x40, 0x06, 0x2F, 0x1A]); // w64_sr_v820_act3_part1_redacted
    register_pkg_key(0x34780A57DB63A65F, [0xD1, 0xCF, 0xED, 0xCC, 0x9B, 0x88, 0x06, 0xE3, 0x37, 0xA5, 0x8B, 0xCE, 0x20, 0xD1, 0x59, 0xCD], [0x31, 0xC3, 0x2C, 0xE2, 0x9C, 0x4A, 0x06, 0x70, 0x42, 0xFD, 0x74, 0xE7]); // w64_sr_v826_cines_redacted
    register_pkg_key(0x77D7CF2792E2061C, [0x89, 0xE7, 0xA6, 0xE7, 0x1B, 0xA7, 0xD2, 0x9D, 0x36, 0x43, 0x2E, 0x34, 0xF7, 0x5F, 0x47, 0xED], [0x4D, 0x62, 0xC3, 0xA7, 0x7E, 0xA0, 0xC8, 0x6C, 0x0D, 0x44, 0xE2, 0x0F]); // w64_sr_v900_raid_redacted
    register_pkg_key(0x80C00E04D7C00B02, [0x89, 0xE7, 0xA6, 0xE7, 0x1B, 0xA7, 0xD2, 0x9D, 0x36, 0x43, 0x2E, 0x34, 0xF7, 0x5F, 0x47, 0xED], [0x4D, 0x62, 0xC3, 0xA7, 0x7E, 0xA0, 0xC8, 0x6C, 0x0D, 0x44, 0xE2, 0x0F]); // w64_raid_gateways_redacted
    register_pkg_key(0xC05CCF0D0A2E56D4, [0x33, 0xB5, 0x14, 0x43, 0x4B, 0x0B, 0x0B, 0x29, 0x6C, 0xFD, 0xAB, 0xF4, 0x89, 0x2E, 0x2F, 0xFD], [0xCE, 0xFE, 0xE0, 0x3D, 0x79, 0x5B, 0x5E, 0x08, 0xB5, 0xCA, 0x23, 0xF5]); // w64_sr_v910_epic_redacted
    register_pkg_key(0xB666BDD480C07F45, [0x46, 0xDF, 0xEE, 0x57, 0x87, 0x4E, 0xD7, 0x48, 0x00, 0x76, 0xA6, 0xC0, 0xC0, 0xD9, 0x2A, 0x77], [0x82, 0x54, 0x6C, 0xF4, 0xD7, 0xFE, 0x46, 0x08, 0xB0, 0xA1, 0xD8, 0xC2]); // w64_sr_v826_offer_redacted
    register_pkg_key(0x8E1C4906BC077FC4, [0x00, 0x4B, 0x0D, 0x78, 0x78, 0xBB, 0x33, 0xC5, 0x3B, 0x35, 0x28, 0xC1, 0x07, 0x7D, 0x63, 0xA8], [0x1F, 0x25, 0xED, 0xAD, 0xA6, 0x4C, 0xB5, 0x75, 0xDD, 0xB8, 0x55, 0xB2]); // w64_sr_v826_upsell_redacted
    register_pkg_key(0x5CDA07ECF428FD01, [0xBA, 0x5E, 0xE5, 0xBA, 0x5B, 0xBD, 0x08, 0xE3, 0xB5, 0x4D, 0x21, 0x01, 0x8F, 0xB9, 0xC9, 0x08], [0x9D, 0xA2, 0xD2, 0x9E, 0xE1, 0xCB, 0x32, 0x61, 0x68, 0xD9, 0x7E, 0xEE]); // w64_sr_v950_dungeon_redacted
    register_pkg_key(0xE8F9B4994C3C5EC0, [0xE5, 0xBC, 0xE7, 0x4A, 0xBA, 0xB1, 0xAB, 0x58, 0x26, 0x6D, 0x2E, 0xBA, 0x86, 0xC1, 0xCC, 0x22], [0xBA, 0x6E, 0xE0, 0x43, 0x12, 0x74, 0xF3, 0x1D, 0xC2, 0x9A, 0x9A, 0xF8]); // w64_sr_v950_redacted
    register_pkg_key(0x361103272915D800, [0x08, 0xC3, 0x78, 0x77, 0x6E, 0xB5, 0x4F, 0x80, 0x7F, 0x94, 0x29, 0xE9, 0xAE, 0x09, 0x33, 0x04], [0x09, 0xEB, 0x94, 0x72, 0x29, 0xBF, 0xC9, 0x02, 0xD7, 0x0C, 0x6A, 0x2C]); // w64_sr_v950_promo_redacted
    register_pkg_key(0x835DC8BF1AFEAB83, [0x56, 0x82, 0xFD, 0xAA, 0xC9, 0xB3, 0x32, 0xE1, 0x24, 0x39, 0x6E, 0x02, 0xBB, 0xE6, 0x23, 0x87], [0x1A, 0x51, 0xC3, 0x3F, 0x39, 0x28, 0xFB, 0x0E, 0x56, 0x29, 0x83, 0x04]); // w64_sr_v950_partner_redacted
    register_pkg_key(0xF10A51CB73ACC4E4, [0xAD, 0xF5, 0x8D, 0x01, 0x08, 0x93, 0x18, 0xC9, 0x11, 0x35, 0xE0, 0x25, 0x67, 0xD1, 0xB7, 0x36], [0x7E, 0x65, 0xB0, 0x26, 0x18, 0x5E, 0x09, 0x85, 0xE1, 0xB7, 0x37, 0x4C]); // w64_sr_v955_partner_redacted
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
