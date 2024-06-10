use destiny_pkg::TagHash;
use glam::{Mat4, Quat, Vec4};
use tiger_parse::{tiger_tag, Pointer, ResourcePointer};

use crate::{
    common::ResourceHash,
    occlusion::{SObjectOcclusionBounds, SOcclusionBounds, AABB},
    statics::SStaticMeshInstances,
    Tag, WideHash, WideTag,
};

#[derive(Debug)]
#[tiger_tag(id = 0x8080891E, size = 0x18)]
// cohae: Shallow read to avoid too many package calls
// TODO(cohae): Implement shallow reading in tiger-parse itself
pub struct SBubbleParentShallow {
    pub file_size: u64,
    // 808091e0
    pub child_map: TagHash,
    pub unkc: u32,

    pub unk10: u64,
    pub map_name: ResourceHash,
}

#[derive(Debug)]
#[tiger_tag(id = 0x8080891E, size = 0x50)]
pub struct SBubbleParent {
    pub file_size: u64,
    pub child_map: Tag<SBubbleDefinition>,
    pub unkc: u32,

    pub unk10: u64,
    pub map_name: ResourceHash,

    #[tag(offset = 0x40)]
    pub unk40: Vec<SUnk808096c9>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x808096C9)]
pub struct SUnk808096c9 {
    pub unk0: u32,
    pub unk4: u32,
    pub unk8: u32,
    pub unkc: u32, // 8080964e
}

// D2Class_01878080
#[derive(Debug)]
#[tiger_tag(id = 0x80808701, size = 0x18)]
pub struct SBubbleDefinition {
    pub file_size: u64,
    pub map_resources: Vec<WideTag<SMapContainer>>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x80808707, size = 0x38)]
pub struct SMapContainer {
    pub file_size: u64,
    #[tag(offset = 0x28)]
    pub data_tables: Vec<TagHash>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x80809883)]
pub struct SMapDataTable {
    pub file_size: u64,
    pub data_entries: Vec<SUnk80809885>,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80809885)]
pub struct SUnk80809885 {
    pub rotation: Quat,      // 0x0
    pub translation: Vec4,   // 0x10
    pub entity_old: TagHash, // 0x20
    pub unk24: u32,
    pub entity: WideHash,
    pub unk38: [u32; 9], //
    pub unk5c: f32,
    pub unk60: f32,
    pub unk64: TagHash,
    pub unk68: ResourceHash,
    pub unk6c: u32,
    pub world_id: u64,
    pub data_resource: ResourcePointer,
    pub unk80: [u32; 4],
}

#[derive(Debug)]
#[tiger_tag(id = 0x80806A0D)]
pub struct SUnk80806ef4 {
    pub unk0: u64,
    pub instances: Tag<SStaticMeshInstances>,
    pub unkc: [u32; 7],
}

/// Terrain
#[derive(Debug)]
#[tiger_tag(id = 0x80806C81, size = 0x88)]
pub struct STerrain {
    pub file_size: u64,
    pub unk8: u64,

    pub unk10: Vec4,
    pub unk20: Vec4,
    pub unk30: Vec4,

    #[tag(offset = 0x50)]
    pub mesh_groups: Vec<SUnk80807154>,

    pub vertex0_buffer: TagHash,
    pub vertex1_buffer: TagHash,
    pub index_buffer: TagHash,
    pub unk_technique1: TagHash,
    pub unk_technique2: TagHash,

    #[tag(offset = 0x78)]
    pub mesh_parts: Vec<SUnk80807152>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x80806C86)]
pub struct SUnk80807154 {
    pub unk0: Vec4,
    pub unk10: f32,
    pub unk14: f32,
    pub unk18: f32,
    pub unk1c: u32,
    pub unk20: Vec4,
    pub unk30: u32,
    pub unk34: u32,
    pub unk38: u32,
    pub unk3c: u32,
    pub unk40: u32,
    pub unk44: u32,
    pub unk48: u32,
    pub unk4c: u32,
    pub dyemap: TagHash,
    pub unk54: u32,
    pub unk58: u32,
    pub unk5c: u32,
}

#[derive(Debug)]
#[tiger_tag(id = 0x80806C84)]
pub struct SUnk80807152 {
    pub technique: TagHash,
    pub index_start: u32,
    pub index_count: u16,
    pub group_index: u8,
    pub detail_level: u8,
}

/// Terrain resource
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff, size = 0x20)]
pub struct SUnk8080714b {
    #[tag(offset = 0x10)]
    pub unk10: u16,
    pub unk12: u16,
    pub unk14: ResourceHash,
    pub terrain: TagHash,
    pub terrain_bounds: TagHash,
}

/// Cubemap volume resource
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff, size = 0x1d0)]
pub struct SCubemapVolume {
    #[tag(offset = 0x20)]
    pub cubemap_extents: Vec4,
    /// Represents the visual center of the cubemap
    pub cubemap_center: Vec4,
    pub unk40: f32,
    pub unk44: [u32; 3],
    pub unk50: Vec4,
    pub unk60: Vec4,

    pub unk70: [u32; 20],

    // Transform matrices?
    pub unkc0: Mat4,
    pub unk100: Vec4,
    pub unk110: Vec4,
    pub unk120: Mat4,

    pub unk160: Vec4,
    pub unk170: Vec4,
    pub unk180: Vec4,
    pub unk190: Vec4,
    pub unk1a0: [u32; 3],

    // TODO(cohae): Removed in TFS, apply versioning to this field
    // pub cubemap_name: Pointer<NullString>,
    pub cubemap_texture: TagHash,      // 0x1ac
    pub _unk_cubemap_skymask: TagHash, // 0x1b0
    pub voxel_ibl_texture: TagHash,    // 0x1b4
    pub unk1c4: [u32; 3],
}

/// Decal collection resource
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80806e68 {
    pub file_size: u64,
    pub instances: Vec<SUnk80806e6c>,
    pub instance_points: TagHash,
    pub unk_vertex_colors: TagHash,

    pub unk30: [u32; 2],
    pub occlusion_bounds: Tag<SOcclusionBounds>,
    _pad3c: u32,
    pub bounds: AABB,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806963)]
pub struct SUnk80806e6c {
    pub material: TagHash,
    pub start: u16,
    pub count: u16,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80806df3 {
    pub file_size: u64,
    pub unk8: Vec<SUnk80806dec>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80806dec {
    pub material: TagHash,
    pub index_buffer: TagHash,
    pub vertex_buffer: TagHash,
    pub unkc: u32,
    pub unk10: [u32; 4],

    pub translation: Vec4,

    pub unk30: Vec4,
    pub unk40: Vec4,
    pub unk50: Vec4,
}

// Unknown resource (some kind of octree?)
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80807268 {
    pub file_size: u64,
    /// Vertex buffer
    pub unk8: TagHash,
    pub unkc: u32,
    pub unk10: Vec<SUnk8080726a>,
    pub unk20: [u32; 6],
    /// Vertex buffer
    pub unk38: TagHash,
    pub unk3c: u32,
    pub unk40: Vec<SUnk8080726a>,
    pub unk50: Vec<SUnk8080726d>,
    pub unk60: Vec<u16>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk8080726a {
    pub unk0: [u32; 4],
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk8080726d {
    pub unk0: Vec4,
    pub unk10: Vec4,
    pub unk20: Vec4,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80809162 {
    pub file_size: u64,
    pub unk8: Vec<SUnk80809164>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80809164 {
    pub unk0: Vec4,
    pub unk10: Vec4,
    pub unk20: [u32; 4],
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80809802 {
    pub file_size: u64,
    pub unk8: TagHash,
    pub unkc: TagHash,
    pub unk10: u32,
    pub unk14: TagHash,
    pub unk18: TagHash,
    pub unk1c: u32,
    pub streams: Vec<TagHash>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806AA7)]
pub struct SUnk80806aa7 {
    pub file_size: u64,
    pub unk8: Vec<SUnk80806aa9>,
    pub unk18: Vec<SObjectOcclusionBounds>,
    pub unk28: Vec<u32>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806AA9)]
pub struct SUnk80806aa9 {
    /// Transformation matrix
    pub transform: [f32; 16],

    /// Same as the bounding box from the SObjectOcclusionBounds array
    pub bounds: AABB,

    pub unk60: Tag<SUnk80806aae>,
    pub unk64: f32,
    pub unk68: f32,
    pub unk6c: i16,
    pub unk6e: u16,

    pub unk70: f32,
    pub unk74: u32,
    pub unk78: TagHash,
    pub unk7c: u32,

    pub unk80: u64,
    pub unk88: u32,
    pub unk8c: u32,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806AAE)]
pub struct SUnk80806aae {
    pub file_size: u64,
    pub entity_model: TagHash,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806C65)]
pub struct SLightCollection {
    pub file_size: u64,
    pub unk8: u64,
    pub bounds: AABB,
    pub unk30: Vec<SLight>,
    pub unk40: Vec<SUnk80809f4f>,
    pub light_count: u32,
    pub unk54: u32,
    pub occlusion_bounds: Tag<SOcclusionBounds>,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806C70, size = 240)]
pub struct SLight {
    pub unk0: Vec4,
    pub unk10: Vec4,
    pub unk20: Vec4,
    pub unk30: Vec4,
    pub unk40: [u32; 4],
    pub unk50: Vec4,
    pub light_to_world: Mat4,
    pub unka0: u32,
    pub unka4: u32,
    pub unka8: u32,
    pub unkac: f32,
    pub unkb0: f32,
    pub unkb4: f32,
    pub unkb8: f32,
    pub unkbc: f32,

    // TODO(cohae): This field is new in TFS. Taghash-like value such as 9E440E84, purpose unknown
    pub unkc0: f32,

    pub technique_shading: TagHash,
    pub technique_volumetrics: TagHash,
    pub technique_compute_lightprobe: TagHash,
    pub unkd0: TagHash, // Unk80806da1
    pub unkd4: TagHash, // Unk80806da1
    pub unkd8: [u32; 6],
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806C71)]
pub struct SShadowingLight {
    pub unk0: Vec4,
    pub unk10: Vec4,
    pub unk20: Vec4,
    pub unk30: Vec4,
    pub unk40: [u32; 4],
    pub unk50: Vec4,
    pub light_to_world: Mat4,
    pub unka0: u32,
    pub unka4: u32,
    pub unka8: u32,
    pub unkac: f32,
    pub unkb0: f32,
    pub unkb4: f32,
    pub unkb8: f32,

    pub unkbc: f32,

    pub far_plane: f32,
    pub half_fov: f32,

    pub unkc8: u32,
    pub unkcc: f32,

    pub technique_shading: TagHash,
    pub technique_shading_shadowing: TagHash,
    pub technique_volumetrics: TagHash,
    pub technique_volumetrics_shadowing: TagHash,
    pub technique_compute_lightprobe: TagHash,
    pub technique_compute_lightprobe_shadowing: TagHash,

    pub unke8: TagHash, // Unk80806da1
    pub unkec: TagHash, // Unk80806da1

    pub unkf0: [f32; 5],
    pub unk104: [u8; 12],
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80809F4F)]
pub struct SUnk80809f4f {
    pub rotation: Quat,
    pub translation: Vec4,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80808CB7)]
pub struct SUnk80808cb7 {
    pub file_size: u64,
    pub unk8: Vec<SRespawnPoint>,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80808CB9)]
pub struct SRespawnPoint {
    pub rotation: Quat,
    pub translation: Vec4,
    pub unk20: u32,
    // cohae: Probably padding
    pub unk24: [u32; 3],
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk808085c2 {
    pub file_size: u64,
    pub unk8: Vec<SUnk808085c4>,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x808085C4)]
pub struct SUnk808085c4 {
    pub unk0: [u32; 4],
    pub unk10: [u32; 4],
    pub translation: Vec4,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80806d19 {
    pub file_size: u64,
    pub unk8: TagHash,
    pub unkc: u32, // Padding
    pub unk10: Vec<()>,
    pub unk20: TagHash,
    pub unk24: u32, // Padding
    pub unk28: Vec<()>,
    pub unk38: TagHash,
    pub unk3c: u32, // Padding
    pub unk40: Vec<()>,
    pub unk50: Vec<SUnk80806d4f>,
    pub unk60: Vec<()>,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806D4F)]
pub struct SUnk80806d4f {
    pub translation: Vec4,
    pub unk10: [u32; 4],
    pub unk20: [u32; 4],
}

//#[derive(Clone, Debug)]
// #[tiger_tag(id = 0xffffffff)]
// pub struct Unk808066a2 {
//     pub file_size: u64,
//     pub unk8: Vec<()>,
//     pub unk18: Vec<()>,
//     /// Havok file
//     pub unk28: TagHash,
// }

#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80809178 {
    // Points to havok pre-tag
    pub unk0: Pointer<SSlipSurfaceVolume>,

    pub unk8: u32,
    pub unkc: u32,
    pub area_name: ResourceHash,
    pub unk14: ResourceHash,
    pub unk18: ResourceHash,

    // Absolute offset to havok pre-tag??
    pub unk1c: u64,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk8080917b {
    // Points to havok pre-tag
    pub unk0: Pointer<SSlipSurfaceVolume>,
    pub unk8: u32,
    pub unkc: u32,
    pub kind: u8,
    pub unk11: u8,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SSlipSurfaceVolume {
    pub unk0: [u32; 4],
    pub havok_file: TagHash,
    pub unk14: u32,
    pub shape_index: u32,
}

#[derive(Clone)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk808068d4 {
    pub unk0: u32,
    pub unk4: u32,
    pub unk8: u32,
    pub unkc: u32,
    pub entity_model: TagHash,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80808604 {
    pub unk0: [u32; 4],
    pub unk10: Tag<SUnk80808724>,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80808606)]
pub struct SUnk80808606 {
    pub rotation: Quat,
    pub translation: Vec4,
    pub unk20: Vec4,
    pub unk30: Vec4,
    pub unk40: u32,
    pub shape_index: u32,
    pub unk48: u32,
    pub unk4c: u32,
    pub unk50: [u32; 4],
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80808724 {
    pub file_size: u64,
    pub unk8: Vec<SUnk80808606>,
    pub havok_file: TagHash,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0x8080824C)]
pub struct SUnk8080824c {
    pub rotation: Quat,
    pub translation: Vec4,
    pub unk20: Vec4,
    pub unk30: Vec4,
    pub unk40: Vec4,
    pub unk50: Vec4,
    pub unk60: Vec<Vec4>,
    pub unk70: Vec<()>,
    pub unk80: f32,
    pub unk84: [u32; 3],
    pub unk90: [u32; 4],
    pub unka0: [u32; 3],
    pub shape_index: u32,
    pub unkb0: [u32; 4],
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80808248 {
    pub file_size: u64,
    pub havok_file: TagHash,
    _pad: u32,
    pub unk10: Vec<SUnk8080824c>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80808246 {
    pub unk0: [u32; 4],
    pub unk10: Tag<SUnk80808248>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80806ac2 {
    pub unk0: [u32; 4],
    pub unk10: Tag<SUnk80806ac4>,
    pub array_index: u32,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SUnk80806ac4 {
    pub file_size: u64,
    pub havok_file: TagHash,
    _pad: u32,
    pub unk10: Vec<SUnk80806ed8>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806ED8)]
pub struct SUnk80806ed8 {
    pub rotation: Quat,
    pub translation: Vec4,
    pub unk20: Vec4,
    pub unk30: Vec4,
    pub unk40: Vec4,
    pub unk50: Vec4,

    pub unk60: Vec<Vec4>,
    pub unk70: Vec<()>,

    pub unk80: f32,
    pub unk84: [u32; 3],
    pub unk90: [u32; 4],
    pub unka0: [u32; 3],
    pub shape_index: u32,
    pub unkb0: u64,
    pub unkb8: Vec<()>,
    pub unkc8: [u32; 2],
    pub unkd0: [u32; 4],
    pub unke0: [u32; 4],
}

#[derive(Debug)]
#[tiger_tag(id = 0x80806BC1)]
pub struct SMapAtmosphere {
    pub unk0: [u32; 32],
    pub lookup_texture_0: WideHash,
    pub lookup_texture_1: WideHash,
    pub lookup_texture_2: WideHash,
    pub lookup_texture_3: WideHash,
}

#[derive(Debug)]
#[tiger_tag(id = 0x80806A78)]
pub struct SLensFlare {
    // TODO(cohae): Placeholder struct
}
