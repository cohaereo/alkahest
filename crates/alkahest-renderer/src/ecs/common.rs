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
}

impl Display for Icon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Icon::Unicode(c) | Icon::Colored(c, _) => write!(f, "{}", c),
        }
    }
}

pub struct Label(pub String);

impl Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Label {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Label {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for Label {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

pub struct Hidden;
pub struct Global;

/// Marker component to indicate that the entity is allowed to be modified in
/// potentially destructive ways (e.g. deleting it, changing it's name, etc.)
pub struct Mutable;

pub struct Water;
