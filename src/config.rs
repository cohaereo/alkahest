use lazy_static::lazy_static;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::warn;

lazy_static! {
    pub static ref CONFIGURATION: RwLock<Config> = RwLock::new(Config::default());
}

pub fn persist() {
    if let Err(e) = std::fs::write(
        "config.yml",
        serde_yaml::to_string(&*CONFIGURATION.read()).expect("Fatal: failed to write config"),
    ) {
        warn!("Failed to write config: {e}");
    }
}

pub fn with<F, T>(f: F) -> T
where
    F: FnOnce(&Config) -> T,
{
    f(&*CONFIGURATION.read())
}

pub fn with_mut<F, T>(f: F) -> T
where
    F: FnOnce(&mut Config) -> T,
{
    f(&mut *CONFIGURATION.write())
}

#[macro_export]
macro_rules! config {
    () => {
        (CONFIGURATION.read())
    };
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub window: WindowConfig,
}

#[derive(Serialize, Deserialize)]
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
