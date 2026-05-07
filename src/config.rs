#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub vsync: bool,
    pub resolution_scale: f32,
    pub framerate_limit: usize,
    pub framelimiter_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            vsync: true,
            resolution_scale: 1.0,
            framerate_limit: 60,
            framelimiter_enabled: false,
        }
    }
}
