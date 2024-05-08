use hecs::Entity;
use smallvec::SmallVec;

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

pub struct Label(pub String);

pub struct Hidden;
pub struct Global;

/// Marker component to indicate that the entity is allowed to be modified in
/// potentially destructive ways (e.g. deleting it, changing it's name, etc.)
pub struct Mutable;

pub struct Water;
