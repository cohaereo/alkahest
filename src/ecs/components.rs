use destiny_pkg::TagHash;

use crate::{
    map_resources::MapResource,
    render::{scopes::ScopeRigidModel, ConstantBuffer},
    structure::ExtendedHash,
    types::AABB,
};

#[derive(Copy, Clone)]
/// Tiger entity world ID
pub struct EntityWorldId(pub u64);

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ResourceOriginType {
    Map,

    Activity,
    Activity2,
}

pub struct ResourcePoint {
    pub entity: ExtendedHash,
    pub resource_type: u32,
    pub resource: MapResource,

    pub has_havok_data: bool,
    /// Does this node belong to an activity?
    pub origin: ResourceOriginType,

    // TODO(cohae): Temporary
    pub entity_cbuffer: ConstantBuffer<ScopeRigidModel>,
}

impl ResourcePoint {
    pub fn entity_key(&self) -> u64 {
        match self.resource {
            MapResource::Unk80806aa3(_, t, _) => t.0 as u64,
            MapResource::Unk808068d4(t) => t.0 as u64,
            _ => self.entity.key(),
        }
    }
}

/// Just a marker for now
pub struct PointLight;

pub struct CubemapVolume(pub TagHash, pub AABB, pub String);

pub struct ActivityGroup(pub u32);

pub struct Label(pub String);
