use std::fmt::Display;

use ecolor::Color32;

/// Tiger entity world ID
#[derive(Copy, Clone)]
pub struct EntityWorldId(pub u64);

#[derive(strum::Display, Copy, Clone, PartialEq, Eq)]
pub enum ResourceOrigin {
    Map,

    Activity,
    ActivityBruteforce,
    Ambient,
}

// pub struct HavokShape(pub TagHash, pub Option<CustomDebugShape>);

pub struct ActivityGroup(pub u32);

#[derive(Clone)]
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

pub struct Label {
    pub label: String,
    pub default: bool,
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
        }
    }
}

impl From<&str> for Label {
    fn from(s: &str) -> Self {
        Self {
            label: s.to_string(),
            default: false,
        }
    }
}

impl From<String> for Label {
    fn from(s: String) -> Self {
        Self {
            label: s,
            default: false,
        }
    }
}

impl AsRef<str> for Label {
    fn as_ref(&self) -> &str {
        &self.label
    }
}

pub struct Hidden;
pub struct Global;
pub struct Ghost;

/// Marker component to indicate that the entity is allowed to be modified in
/// potentially destructive ways (e.g. deleting it, changing it's name, etc.)
pub struct Mutable;

pub struct Water;
