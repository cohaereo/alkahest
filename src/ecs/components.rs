use std::ops::{Deref, DerefMut};

use alkahest_data::{occlusion::AABB, ExtendedHash};
use destiny_pkg::TagHash;
use glam::{Vec3, Vec4};

use crate::{
    map_resources::MapResource,
    render::{
        cbuffer::ConstantBufferCached, scopes::ScopeRigidModel, EntityRenderer, InstancedRenderer,
        TerrainRenderer,
    },
};

#[derive(Copy, Clone)]
/// Tiger entity world ID
pub struct EntityWorldId(pub u64);

#[derive(strum::Display, Copy, Clone, PartialEq, Eq)]
pub enum ResourceOriginType {
    Map,

    Activity,
    ActivityBruteforce,
    Ambient,
}

pub struct ResourcePoint {
    pub entity: ExtendedHash,
    pub resource_type: u32,
    pub resource: MapResource,

    pub has_havok_data: bool,
    /// Does this node belong to an activity?
    pub origin: ResourceOriginType,

    // TODO(cohae): Temporary
    pub entity_cbuffer: ConstantBufferCached<ScopeRigidModel>,
}

impl ResourcePoint {
    pub fn entity_key(&self) -> u64 {
        match self.resource {
            MapResource::Unk80806aa3(_, t) => t.0 as u64,
            MapResource::Unk808068d4(t) => t.0 as u64,
            _ => self.entity.key(),
        }
    }
}

pub struct PointLight {
    pub attenuation: Vec4,
}

// pub struct HavokShape(pub TagHash, pub Option<CustomDebugShape>);

pub struct CubemapVolume(pub TagHash, pub AABB, pub String);

pub struct ActivityGroup(pub u32);

pub struct Label(pub String);

// TODO(cohae): This is currently only used for user-spawned entities, but it should be used for resource points as well
pub struct EntityModel(
    pub EntityRenderer,
    pub ConstantBufferCached<ScopeRigidModel>,
    pub TagHash,
);

pub struct Terrain(pub TerrainRenderer);

pub struct StaticInstances(pub InstancedRenderer, pub TagHash);

pub struct Water;

macro_rules! bool_trait {
    ($name: ident) => {
        pub struct $name(pub bool);

        impl Deref for $name {
            type Target = bool;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}

bool_trait!(Visible);
bool_trait!(Global);

pub struct Ruler {
    pub start: Vec3,
    pub end: Vec3,
    pub color: [u8; 3],
    pub rainbow: bool,
    pub scale: f32,
    pub marker_interval: f32,
    pub show_individual_axis: bool,
}

impl Default for Ruler {
    fn default() -> Self {
        Self {
            start: Vec3::ZERO,
            end: Vec3::ZERO,
            color: [255, 255, 255],
            rainbow: false,
            scale: 1.0,
            marker_interval: 0.0,
            show_individual_axis: false,
        }
    }
}

impl Ruler {
    pub fn length(&self) -> f32 {
        (self.start - self.end).length()
    }

    pub fn direction(&self) -> Vec3 {
        (self.end - self.start).normalize()
    }
}

pub struct Sphere {
    pub detail: u8,
    pub color: [u8; 4],
    pub rainbow: bool,
}

impl Default for Sphere {
    fn default() -> Self {
        Self {
            detail: 4,
            color: [255, 255, 255, 80],
            rainbow: false,
        }
    }
}

pub struct Beacon {
    pub color: [u8; 3],
    pub freq: f32,
    pub distance: f32,
    pub travel_time: f32,
}

impl Default for Beacon {
    fn default() -> Self {
        Self {
            color: [255, 255, 255],
            freq: 1.0,
            distance: 0.5,
            travel_time: 0.7,
        }
    }
}
/// Marker component to indicate that the entity is allowed to be modified in potentially destructive ways
/// (e.g. deleting it, changing it's name, etc.)
pub struct Mutable;

/// Marker component to indicate that the entity is a light
pub struct Light;
