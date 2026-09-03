use std::collections::HashSet;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub vsync: bool,
    pub resolution_scale: f32,
    pub framerate_limit: usize,
    pub framelimiter_enabled: bool,
    pub visual: VisualSettings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            vsync: true,
            resolution_scale: 1.0,
            framerate_limit: 60,
            framelimiter_enabled: false,
            visual: VisualSettings::default(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(default)]
pub struct VisualSettings {
    pub node_nametags: bool,
    pub node_nametags_named_only: bool,
    pub node_filters: HashSet<String>,
    pub node_nametags_distance_limit: bool,
    pub node_nametags_max_distance: f32,
}

impl Default for VisualSettings {
    fn default() -> Self {
        Self {
            node_nametags: false,
            node_nametags_named_only: false,
            node_filters: crate::world::node_filter::NodeFilter::ALL
                .iter()
                .map(|f| f.to_string())
                .collect(),
            node_nametags_distance_limit: true,
            node_nametags_max_distance: 2000.0,
        }
    }
}
