use std::fmt;

use glam::Vec3;

use crate::world::node_filter::NodeFilter;

#[derive(Debug, Clone)]
pub struct Label {
    pub label: String,
    pub kind: NodeFilter,
    pub default: bool,
    pub offset: Vec3,
}

impl Label {
    pub fn default_for(kind: NodeFilter) -> Self {
        Self {
            label: kind.name().to_string(),
            kind,
            default: true,
            offset: Vec3::ZERO,
        }
    }

    pub fn named(name: impl Into<String>, kind: NodeFilter) -> Self {
        Self {
            label: name.into(),
            kind,
            default: false,
            offset: Vec3::ZERO,
        }
    }

    pub fn with_offset(mut self, offset: Vec3) -> Self {
        self.offset = offset;
        self
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}
