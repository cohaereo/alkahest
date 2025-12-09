use assert_offset::AssertOffsets;
use glam::{Quat, Vec4};
use tiger_parse::{tiger_type, tiger_variant_enum, FnvHash, OptionalVariantPointer, Padding};
use tiger_pkg::TagHash;

use crate::{
    tag::{OptionalTag, Tag, WideHash, WideTag},
    tfx::features::{
        cubemap::SCubemapComponent,
        decals::SDecalCollection,
        decorators::SDecorator,
        light::{SLightCollection, SShadowingLight},
        sky_objects::SSkyObjectCollection,
    },
};

#[derive(Debug, AssertOffsets)]
#[tiger_type(id = 0x8080891E, size = 0x50)]
pub struct SBubbleParent {
    pub file_size: u64,

    pub definition: WideTag<SBubbleDefinition>, // WideHash as of 8.2.0
    // pub unkc: Padding<4>, // Removed in 8.2.0
    pub unk10: u64,
    pub map_name: FnvHash,
}

#[derive(Debug, AssertOffsets)]
#[tiger_type(id = 0x80808701, size = 0x50)]
pub struct SBubbleDefinition {
    pub file_size: u64,
    pub containers: Vec<WideTag<SMapContainer>>,
}

#[derive(Debug)]
#[tiger_type(id = 0x80808707, size = 0x38)]
pub struct SMapContainer {
    pub file_size: u64,
    #[tiger(offset = 0x28)]
    pub data_tables: Vec<TagHash>,
}

#[tiger_type(id = 0x80809883)]
pub struct SMapNodeTable {
    pub file_size: u64,
    pub nodes: Vec<SMapNodeEntry>,
}

#[tiger_type(id = 0x80809885)]
pub struct SMapNodeEntry {
    pub rotation: Quat,
    pub translation: Vec4,
    pub entity_old: TagHash,
    pub unk24: u32,
    pub entity: WideHash,
    pub unk38: [u32; 9], //
    pub unk5c: f32,
    pub unk60: f32,
    pub unk64: TagHash,
    pub unk68: FnvHash,
    pub unk6c: u32,
    pub world_id: u64,
    pub primary_component_data: OptionalVariantPointer<ComponentData>,
    pub unk80: [u32; 4],
}

tiger_variant_enum! {
    [offset = 0x10]
    [Unknown(true)]
    enum ComponentData {
        SStaticTerrainPatchesComponent,
        SStaticInstancesCollectionComponent
        // SSkyObjectCollectionComponent,
        // SDecalCollectionComponent,
        // SDecoratorsComponent,
        // SMaterialPermutationsComponent,
        // SShadowingLightComponent,
        // SLightCollectionComponent,
        // SCubemapComponent
    }
}

#[tiger_type(id = 0x8080402E)]
pub struct SMaterialPermutationsComponent {
    pub config: Vec<(u32, u32)>,
}

// #[tiger_type(id = 0x80806F38)]
// pub struct SStaticAmbientOcclusionComponent {
//     pub ao: Tag<SStaticAmbientOcclusion>,
// }

#[tiger_type(id = 0x80806CC9)]
pub struct SStaticInstancesCollectionComponent {
    pub instances: TagHash,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80806C7D)]
pub struct SStaticTerrainPatchesComponent {
    pub identifier: u64,
    pub terrain: TagHash,
    pub terrain_bounds: TagHash,
}

#[tiger_type(id = 0x80808378)]
pub struct SSkyObjectCollectionComponent {
    pub objects: OptionalTag<SSkyObjectCollection>,
}

// #[tiger_type(id = 0x80806DE0)]
// pub struct SWaterPlaneComponent {
//     pub model: TagHash,
// }

#[tiger_type(id = 0x80808335)]
pub struct SLightCollectionComponent {
    pub lights: OptionalTag<SLightCollection>,
}

#[tiger_type(id = 0x80808544)]
pub struct SShadowingLightComponent {
    pub light: OptionalTag<SShadowingLight>,
}

#[tiger_type(id = 0x8080821E)]
pub struct SDecalCollectionComponent {
    pub decals: OptionalTag<SDecalCollection>,
}

#[tiger_type(id = 0x808085AA)]
pub struct SDecoratorsComponent {
    pub decorators: OptionalTag<SDecorator>,
}
