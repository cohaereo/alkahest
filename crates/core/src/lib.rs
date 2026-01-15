pub mod convar;
use std::{path::PathBuf, sync::Arc};

use anyhow::Context;
pub use convar::*;
use tiger_pkg::PackageManager;
use tracing::{error, info};
pub mod config;
pub mod job;

#[cfg(feature = "panic-hook")]
pub mod panic_hook;

#[cfg(feature = "panic-hook")]
pub use panic_hook::setup_panic_hook;

pub const DESTINY2_APP_ID: u64 = 1085660;

pub fn initialize_package_manager<'a>(
    suggested_path: impl Into<Option<&'a str>>,
) -> anyhow::Result<()> {
    let game_path = if let Some(path) = &suggested_path.into() {
        path.to_string()
    } else {
        let Some(steamapp) = game_detector::steam::get_all_apps()
            .context("Failed to enumerate Steam apps")?
            .into_iter()
            .find(|a| a.appid == DESTINY2_APP_ID)
        else {
            error!(
                "Failed to find Destiny 2 app in Steam library. If you don't have Destiny 2 \
                 installed through Steam, then you can specify the path to the game directory \
                 using the --gamedir/-g argument."
            );
            return Ok(());
        };

        info!("Found Destiny 2 installation at '{}'", steamapp.game_path);

        steamapp.game_path
    };

    let pm = Arc::new(
        PackageManager::new(
            PathBuf::from(&game_path).join("packages"),
            tiger_pkg::GameVersion::Destiny(tiger_pkg::DestinyVersion::Destiny2TheEdgeOfFate),
            None,
        )
        .context("Failed to initialize package manager")?,
    );
    tiger_pkg::initialize_package_manager(&pm);
    Ok(())
}
