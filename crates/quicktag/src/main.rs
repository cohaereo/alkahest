mod cache;
mod gui;
mod packages;
mod references;
mod scanner;
mod tagtypes;
mod text;

use std::{path::PathBuf, str::FromStr, sync::Arc};

use destiny_pkg::{PackageManager, PackageVersion};
use eframe::IconData;
use log::info;
use packages::PACKAGE_MANAGER;

use crate::gui::QuickTagApp;

fn main() -> eframe::Result<()> {
    env_logger::init();

    info!("Initializing package manager");
    let pm = PackageManager::new(
        PathBuf::from_str(&std::env::args().nth(1).unwrap()).unwrap(),
        PackageVersion::Destiny2Lightfall,
    )
    .unwrap();

    *PACKAGE_MANAGER.write() = Some(Arc::new(pm));

    let native_options = eframe::NativeOptions {
        initial_window_size: Some([400.0, 300.0].into()),
        min_window_size: Some([300.0, 220.0].into()),
        icon_data: Some(
            IconData::try_from_png_bytes(include_bytes!("../quicktag.png"))
                .expect("Failed to load icon"),
        ),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "QuickTag",
        native_options,
        Box::new(|cc| Box::new(QuickTagApp::new(cc))),
    )
}
