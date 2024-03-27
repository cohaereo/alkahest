use directories::ProjectDirs;
use egui::epaint::ahash::HashMap;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

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
        (CONFIGURATION.read())
    };
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub window: WindowConfig,
    pub resources: ResourceConfig,
    pub render_settings: RenderConfig,

    pub update_channel: Option<UpdateChannel>,
    pub packages_directory: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct ResourceConfig {
    pub show_resources: bool,
    pub resource_distance_limit: bool,
    pub map_resource_label_background: bool,
    pub filters: HashMap<String, bool>,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            resource_distance_limit: true,
            map_resource_label_background: true,
            show_resources: false,
            filters: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RenderConfig {
    pub draw_crosshair: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub maximised: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            width: 1600,
            height: 900,
            pos_x: 0,
            pos_y: 0,
            maximised: false,
        }
    }
}
