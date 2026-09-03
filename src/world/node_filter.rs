use std::fmt;

use egui::Color32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeFilter {
    Entity,
    RespawnPoint,
}

impl NodeFilter {
    pub const ALL: &'static [NodeFilter] = &[NodeFilter::Entity, NodeFilter::RespawnPoint];

    pub fn name(&self) -> &'static str {
        match self {
            NodeFilter::Entity => "Entities",
            NodeFilter::RespawnPoint => "Respawn Points",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            NodeFilter::Entity => Color32::WHITE,
            NodeFilter::RespawnPoint => Color32::from_rgb(220, 20, 20),
        }
    }
}

impl fmt::Display for NodeFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
