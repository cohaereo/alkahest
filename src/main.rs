#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#[macro_use]
extern crate tracing;
use std::{fs::File, rc::Rc, sync::Mutex};

use anyhow::Context;
use app::App;
use clap::Parser;
use cli::AppArgs;
use itertools::Itertools;
use tracing_subscriber::{
    Layer,
    filter::{EnvFilter, LevelFilter, Targets},
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

use crate::cli::print_banner;

mod app;
#[cfg(feature = "wwise")]
mod audio;
mod cli;
mod config;
mod task;
mod ui;
mod updater;
mod world;

#[cfg(all(feature = "dhat-heap", not(feature = "tracy")))]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[cfg(feature = "tracy-alloc")]
#[global_allocator]
static GLOBAL: tracy_client::ProfiledAllocator<std::alloc::System> =
    tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

fn main() -> anyhow::Result<()> {
    dioxus_devtools::connect_subsecond();
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    fix_windows_console();
    alkahest_core::setup_panic_hook();
    init_tracing()?;

    print_banner();

    let args = AppArgs::parse();

    alkahest_core::initialize_package_manager(args.gamedir.as_deref())?;

    let sdl_context = Rc::new(sdl3::init().expect("Failed to initialize SDL"));
    #[cfg(target_os = "linux")]
    sdl3::hint::set("SDL_VIDEO_DRIVER", "x11");

    let video_subsystem = sdl_context
        .video()
        .expect("Failed to initialize video subsystem");

    let mut window = {
        let mut builder = video_subsystem.window("Alkahest", 1920, 1080);

        let mut builder_ref = builder.position_centered().resizable().maximized();

        if cfg!(not(target_os = "windows")) {
            builder_ref = builder_ref.vulkan();
        }

        builder_ref.build().expect("Failed to create window")
    };

    if let Some(display_index) = args.display {
        let displays = video_subsystem.displays()?;
        let Some(display) = displays.get(display_index) else {
            anyhow::bail!(
                "Invalid display index (available displays: {:?})",
                displays.iter().enumerate().map(|(i, _d)| i).collect_vec()
            );
        };
        let display_center = display.get_bounds()?.center();
        let window_size = window.size();
        window.set_position(
            sdl3::video::WindowPos::Positioned(display_center.x - window_size.0 as i32 / 2),
            sdl3::video::WindowPos::Positioned(display_center.y - window_size.1 as i32 / 2),
        );
    }

    let mut app = App::new(sdl_context.clone(), Rc::new(window), args)?;
    let mut event_pump = sdl_context.event_pump().unwrap();
    'app: while app.running {
        for event in event_pump.poll_iter() {
            match event {
                sdl3::event::Event::Quit { .. } => break 'app,
                _ => app.handle_event(event),
            }
        }

        app.render(&event_pump);
    }

    tiger_pkg::finalize_package_manager();

    Ok(())
}

fn init_tracing() -> anyhow::Result<()> {
    let log_file = File::create("alkahest.log").context("creating log file")?;

    let file_filter = Targets::new()
        .with_default(LevelFilter::DEBUG)
        .with_target("gdt_cpus", LevelFilter::OFF)
        .with_target("ureq", LevelFilter::OFF)
        .with_target("rustls", LevelFilter::OFF);

    let file_layer = fmt::layer()
        .with_file(false)
        .with_ansi(false)
        .with_writer(Mutex::new(log_file))
        .with_filter(file_filter);

    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let stderr_layer = fmt::layer()
        .with_file(false)
        .with_writer(std::io::stderr)
        .with_filter(env_filter);

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stderr_layer)
        .init();

    Ok(())
}

fn fix_windows_console() {
    #[cfg(target_os = "windows")]
    {
        pub type Handle = *mut std::ffi::c_void;

        unsafe extern "C" {
            fn SetConsoleMode(handle: Handle, mode: u32) -> i32;
            fn GetStdHandle(handle: u32) -> Handle;
        }

        const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
        const ENABLE_PROCESSED_OUTPUT: u32 = 1u32;
        const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 4u32;
        unsafe {
            let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
            if !stdout.is_null() {
                SetConsoleMode(
                    stdout,
                    ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
                );
            }
        }
    }
}

// Workaround for subsecond missing this symbol while linking (even though its not used)
#[unsafe(no_mangle)]
#[cfg(not(target_os = "windows"))]
extern "C" fn CoCreateGuid() {}
