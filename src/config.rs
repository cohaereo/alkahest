#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub vsync: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { vsync: true }
    }
}
