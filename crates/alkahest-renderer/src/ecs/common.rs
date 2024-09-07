use std::fmt::Display;

use bevy_ecs::{bundle::Bundle, component::Component};
use ecolor::Color32;
use glam::Vec3;

use super::visibility::VisibilityBundle;

/// Tiger entity world ID
#[derive(Component, Copy, Clone)]
pub struct EntityWorldId(pub u64);

#[derive(Component, strum::Display, Copy, Clone, PartialEq, Eq)]
pub enum ResourceOrigin {
    Map,

    Activity,
    ActivityBruteforce,
    Ambient,
}

// pub struct HavokShape(pub TagHash, pub Option<CustomDebugShape>);

pub struct ActivityGroup(pub u32);

#[derive(Component, Clone)]
pub enum Icon {
    Unicode(char),
    Colored(char, Color32),
    // Image(...)
}

impl Icon {
    pub fn color(&self) -> Color32 {
        match self {
            Icon::Unicode(_) => Color32::WHITE,
            Icon::Colored(_, c) => *c,
        }
    }

    pub fn char(&self) -> char {
        match self {
            Icon::Unicode(c) => *c,
            Icon::Colored(c, _) => *c,
        }
    }
}

impl Display for Icon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Icon::Unicode(c) | Icon::Colored(c, _) => write!(f, "{}", c),
        }
    }
}

#[derive(Component)]
pub struct Label {
    pub label: String,
    pub default: bool,
    pub offset: Vec3,
}

impl Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

impl Label {
    pub fn new_default(str: impl AsRef<str>) -> Self {
        let s = str.as_ref();
        Self {
            label: s.to_string(),
            default: true,
            offset: Vec3::new(0.0, 0.0, 0.0),
        }
    }
    pub fn with_offset(mut self, x: f32, y: f32, z: f32) -> Self {
        self.offset = Vec3::new(x, y, z);
        self
    }
}

impl From<&str> for Label {
    fn from(s: &str) -> Self {
        Self {
            label: s.to_string(),
            default: false,
            offset: Vec3::new(0.0, 0.0, 0.0),
        }
    }
}

impl From<String> for Label {
    fn from(s: String) -> Self {
        Self {
            label: s,
            default: false,
            offset: Vec3::new(0.0, 0.0, 0.0),
        }
    }
}

impl AsRef<str> for Label {
    fn as_ref(&self) -> &str {
        &self.label
    }
}

#[derive(Component)]
pub struct Global;

/// Marker component to indicate that the entity is allowed to be modified in
/// potentially destructive ways (e.g. deleting it, changing it's name, etc.)
#[derive(Component)]
pub struct Mutable;

#[derive(Component)]
pub struct Water;

/// Components common to objects that can be rendered
#[derive(Bundle, Default)]
pub struct RenderCommonBundle {
    visibility: VisibilityBundle,
}
