#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub vsync: bool,
    pub resolution_scale: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            vsync: true,
            resolution_scale: 1.0,
        }
    }
}
