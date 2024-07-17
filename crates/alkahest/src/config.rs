use alkahest_renderer::{ecs::tags::NodeFilter, renderer::RendererSettings};
use directories::ProjectDirs;
use egui::ahash::HashSet;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use crate::{updater::UpdateChannel, util::RwLock};

lazy_static! {
    pub static ref CONFIGURATION: RwLock<Config> = RwLock::new(Config::default());
    pub static ref APP_DIRS: ProjectDirs = {
        let pd = ProjectDirs::from("net", "cohaereo", "Alkahest")
            .expect("Failed to get application directories");
        std::fs::create_dir_all(pd.config_dir()).expect("Failed to create config directory");
        std::fs::create_dir_all(pd.config_local_dir())
            .expect("Failed to create local config directory");

        pd
    };
}

pub fn persist() {
    if let Err(e) = std::fs::write(
        APP_DIRS.config_dir().join("config.yml"),
        serde_yaml::to_string(&*CONFIGURATION.read()).expect("Fatal: failed to write config"),
    ) {
        error!("Failed to write config: {e}");
    } else {
        info!("Config written successfully!");
    }
}

pub fn load() {
    if let Ok(c) = std::fs::read_to_string(APP_DIRS.config_dir().join("config.yml")) {
        match serde_yaml::from_str(&c) {
            Ok(config) => {
                with_mut(|c| *c = config);
            }
            Err(e) => {
                error!("Failed to parse config: {e}");
            }
        }
    } else {
        info!("No config found, creating a new one");
        persist();
    }
}

pub fn with<F, T>(f: F) -> T
where
    F: FnOnce(&Config) -> T,
{
    f(&CONFIGURATION.read())
}

pub fn with_mut<F, T>(f: F) -> T
where
    F: FnOnce(&mut Config) -> T,
{
    f(&mut CONFIGURATION.write())
}

#[macro_export]
macro_rules! config {
    () => {
        ($crate::config::CONFIGURATION.read())
    };
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub window: WindowConfig,
    pub renderer: RendererSettings,
    pub visual: VisualSettings,
    pub update_channel: Option<UpdateChannel>,
    pub packages_directory: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct VisualSettings {
    pub draw_crosshair: bool,
    pub node_nametags: bool,
    pub node_nametags_named_only: bool,
    pub node_filters: HashSet<String>,
}

impl Default for VisualSettings {
    fn default() -> Self {
        Self {
            draw_crosshair: false,
            node_nametags: false,
            node_nametags_named_only: false,
            node_filters: NodeFilter::iter()
                .filter_map(|nf| {
                    if !matches!(
                        nf,
                        NodeFilter::PlayerContainmentVolume
                            | NodeFilter::SlipSurfaceVolume
                            | NodeFilter::InstakillBarrier
                            | NodeFilter::Cubemap
                            | NodeFilter::NamedArea
                    ) {
                        Some(nf.to_string())
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub maximised: bool,
    pub fullscreen: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            width: 1600,
            height: 900,
            pos_x: 0,
            pos_y: 0,
            maximised: false,
            fullscreen: false,
        }
    }
}
