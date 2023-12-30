use crate::util::{exe_relative_path, RwLock};
use egui::epaint::ahash::HashMap;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

lazy_static! {
    pub static ref CONFIGURATION: RwLock<Config> = RwLock::new(Config::default());
}

pub fn persist() {
    if let Err(e) = std::fs::write(
        exe_relative_path("config.yml"),
        serde_yaml::to_string(&*CONFIGURATION.read()).expect("Fatal: failed to write config"),
    ) {
        error!("Failed to write config: {e}");
    } else {
        info!("Config written successfully!");
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
