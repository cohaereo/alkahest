use crate::{
    map::ExtendedHash,
    map_resources::MapResource,
    render::{scopes::ScopeRigidModel, ConstantBuffer},
};

#[derive(Copy, Clone)]
/// Tiger entity world ID
pub struct EntityWorldId(pub u64);

pub struct ResourcePoint {
    pub entity: ExtendedHash,
    pub has_havok_data: bool,
    pub resource_type: u32,
    pub resource: MapResource,

    // TODO(cohae): Temporary
    pub entity_cbuffer: ConstantBuffer<ScopeRigidModel>,
}

impl ResourcePoint {
    pub fn entity_key(&self) -> u64 {
        match self.resource {
            MapResource::Unk80806aa3(_, t, _) => t.0 as u64,
            _ => self.entity.key(),
        }
    }
}
