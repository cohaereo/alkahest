use destiny_pkg::TagHash;
use tiger_parse::{tiger_tag, NullString, Pointer, ResourcePointer};

use crate::{
    common::ResourceHash,
    occlusion::{SObjectOcclusionBounds, SOcclusionBounds, AABB},
    statics::SStaticMeshInstances,
    ExtendedHash, ExtendedTag, Tag,
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
    // 808091e0
    pub child_map: Tag<SBubbleDefinition>,
    pub unkc: u32,

    pub unk10: u64,
    pub map_name: ResourceHash,

    #[tag(offset = 0x40)]
    pub unk40: Vec<Unk80809644>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x808096C9)]
pub struct Unk80809644 {
    pub unk0: u32,
    pub unk4: u32,
    pub unk8: u32,
    pub unkc: u32, // 8080964e
}

// D2Class_01878080
#[derive(Debug)]
#[tiger_tag(id = 0xffffffff, size = 0x18)]
pub struct SBubbleDefinition {
    pub file_size: u64,
    pub map_resources: Vec<ExtendedTag<SMapContainer>>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x80808707, size = 0x38)]
pub struct SMapContainer {
    pub file_size: u64,
    #[tag(offset = 0x28)]
    pub data_tables: Vec<Tag<SMapDataTable>>,
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
    pub rotation: glam::Quat,    // 0x0
    pub translation: glam::Vec4, // 0x10
    pub entity_old: TagHash,     // 0x20
    pub unk24: u32,
    pub entity: ExtendedHash,
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
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80806ef4 {
    pub unk0: u64,
    pub instances: Tag<SStaticMeshInstances>,
    pub unkc: [u32; 7],
}

/// Terrain
#[derive(Debug)]
#[tiger_tag(id = 0xffffffff, size = 0x88)]
pub struct STerrain {
    pub file_size: u64,
    pub unk8: u64,

    pub unk10: glam::Vec4,
    pub unk20: glam::Vec4,
    pub unk30: glam::Vec4,

    #[tag(offset = 0x50)]
    pub mesh_groups: Vec<Unk80807154>,

    pub vertex_buffer: TagHash,
    pub vertex_buffer2: TagHash,
    pub indices: TagHash,
    pub material1: TagHash,
    pub material2: TagHash,

    #[tag(offset = 0x78)]
    pub mesh_parts: Vec<Unk80807152>,
}

#[derive(Debug)]
#[tiger_tag(id = 0x80806C86)]
pub struct Unk80807154 {
    pub unk0: glam::Vec4,
    pub unk10: f32,
    pub unk14: f32,
    pub unk18: f32,
    pub unk1c: u32,
    pub unk20: glam::Vec4,
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
pub struct Unk80807152 {
    pub material: TagHash,
    pub index_start: u32,
    pub index_count: u16,
    pub group_index: u8,
    pub detail_level: u8,
}

/// Terrain resource
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff, size = 0x20)]
pub struct Unk8080714b {
    #[tag(offset = 0x10)]
    pub unk10: u16,
    pub unk12: u16,
    pub unk14: ResourceHash,
    pub terrain: TagHash,
    pub terrain_bounds: TagHash,
}

/// Cubemap volume resource
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff, size = 0x1e0)]
pub struct Unk80806b7f {
    #[tag(offset = 0x20)]
    pub cubemap_extents: glam::Vec4,
    /// Represents the visual center of the cubemap
    pub cubemap_center: glam::Vec4,
    pub unk40: f32,
    pub unk44: [u32; 3],
    pub unk50: glam::Vec4,
    pub unk60: glam::Vec4,

    pub unk70: [u32; 20],

    // Transform matrices?
    pub unkc0: [glam::Vec4; 4],
    pub unk100: [glam::Vec4; 4],

    pub unk140: [u32; 28],

    pub cubemap_name: Pointer<NullString>,
    pub cubemap_texture: TagHash,
    pub unk1bc: u32,
    pub unk1c0: TagHash,
    pub unk1c4: [u32; 7],
}

/// Decal collection resource
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80806e68 {
    pub file_size: u64,
    pub instances: Vec<Unk80806e6c>,
    pub transforms: Vec<glam::Vec4>, // 80806e6d
    pub instance_points: TagHash,
    pub unk_vertex_colors: TagHash,

    pub unk30: [u32; 2],
    pub occlusion_bounds: Tag<SOcclusionBounds>,
    _pad3c: u32,
    pub bounds: AABB,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806963)]
pub struct Unk80806e6c {
    pub material: TagHash,
    pub start: u16,
    pub count: u16,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80806df3 {
    pub file_size: u64,
    pub unk8: Vec<Unk80806dec>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80806dec {
    pub material: TagHash,
    pub index_buffer: TagHash,
    pub vertex_buffer: TagHash,
    pub unkc: u32,
    pub unk10: [u32; 4],

    pub translation: glam::Vec4,

    pub unk30: glam::Vec4,
    pub unk40: glam::Vec4,
    pub unk50: glam::Vec4,
}

// Unknown resource (some kind of octree?)
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80807268 {
    pub file_size: u64,
    /// Vertex buffer
    pub unk8: TagHash,
    pub unkc: u32,
    pub unk10: Vec<Unk8080726a>,
    pub unk20: [u32; 6],
    /// Vertex buffer
    pub unk38: TagHash,
    pub unk3c: u32,
    pub unk40: Vec<Unk8080726a>,
    pub unk50: Vec<Unk8080726d>,
    pub unk60: Vec<u16>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk8080726a {
    pub unk0: [u32; 4],
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk8080726d {
    pub unk0: glam::Vec4,
    pub unk10: glam::Vec4,
    pub unk20: glam::Vec4,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80809162 {
    pub file_size: u64,
    pub unk8: Vec<Unk80809164>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80809164 {
    pub unk0: glam::Vec4,
    pub unk10: glam::Vec4,
    pub unk20: [u32; 4],
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80809802 {
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
pub struct Unk80806aa7 {
    pub file_size: u64,
    pub unk8: Vec<Unk80806aa9>,
    pub unk18: Vec<SObjectOcclusionBounds>,
    pub unk28: Vec<u32>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806AA9)]
pub struct Unk80806aa9 {
    /// Transformation matrix
    pub transform: [f32; 16],

    /// Same as the bounding box from the SObjectOcclusionBounds array
    pub bounds: AABB,

    pub unk60: Tag<Unk80806aae>,
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
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80806aae {
    pub file_size: u64,
    pub entity_model: TagHash,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SLightCollection {
    pub file_size: u64,
    pub unk8: u64,
    pub bounds: AABB,
    pub unk30: Vec<SLight>,
    pub unk40: Vec<Unk80809f4f>,
    pub light_count: u32,
    pub unk54: u32,
    pub occlusion_bounds: Tag<SOcclusionBounds>,
}

// 706C8080
#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806C70)]
pub struct SLight {
    pub unk0: glam::Vec4,
    pub unk10: glam::Vec4,
    pub unk20: glam::Vec4,
    pub unk30: glam::Vec4,
    pub unk40: [u32; 4],
    pub unk50: glam::Vec4,
    pub unk60: glam::Mat4,
    pub unka0: u32,
    pub unka4: u32,
    pub unka8: u32,
    pub unkac: f32,
    pub unkb0: f32,
    pub unkb4: f32,
    pub unkb8: f32,
    pub unkbc: f32,

    pub technique_shading: TagHash,
    pub technique_volumetrics: TagHash,
    pub technique_compute_lightprobe: TagHash,
    pub unkcc: TagHash, // Unk80806da1
    pub unkd0: TagHash, // Unk80806da1
    pub unkd4: [u32; 7],
}

// 716C8080
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct SShadowingLight {
    pub unk0: glam::Vec4,
    pub unk10: glam::Vec4,
    pub unk20: glam::Vec4,
    pub unk30: glam::Vec4,
    pub unk40: [u32; 4],
    pub unk50: glam::Vec4,
    pub unk60: glam::Mat4,
    pub unka0: u32,
    pub unka4: u32,
    pub unka8: u32,
    pub unkac: f32,
    pub unkb0: f32,
    pub unkb4: f32,
    pub unkb8: f32,

    // Might be FoV?
    pub unkbc: f32,

    pub unkc0: f32,
    pub unkc4: f32,
    pub unkc8: f32,
    pub unkcc: f32,

    pub technique_shading: TagHash,
    pub technique_shading_shadow: TagHash,
    pub technique_volumetrics: TagHash,
    pub technique_volumetrics_shadow: TagHash,
    pub technique_compute_lightprobe: TagHash,
    pub technique_compute_lightprobe_shadow: TagHash,

    pub unke8: TagHash, // Unk80806da1
    pub unkec: TagHash, // Unk80806da1

    pub unkd0: [u32; 8],
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80809F4F)]
pub struct Unk80809f4f {
    pub rotation: glam::Quat,
    pub translation: glam::Vec4,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80808cb7 {
    pub file_size: u64,
    pub unk8: Vec<Unk80808cb9>,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80808CB9)]
pub struct Unk80808cb9 {
    pub rotation: glam::Quat,
    pub translation: glam::Vec4,
    pub unk20: u32,
    // cohae: Probably padding
    pub unk24: [u32; 3],
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk808085c2 {
    pub file_size: u64,
    pub unk8: Vec<Unk808085c4>,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x808085C4)]
pub struct Unk808085c4 {
    pub unk0: [u32; 4],
    pub unk10: [u32; 4],
    pub translation: glam::Vec4,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80806d19 {
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
    pub unk50: Vec<Unk80806d4f>,
    pub unk60: Vec<()>,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806D4F)]
pub struct Unk80806d4f {
    pub translation: glam::Vec4,
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
#[derive(Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80806c98 {
    pub file_size: u64,
    pub unk8: Vec<TagHash>,
    pub unk18: Vec<u32>,
    pub unk28: Vec<u32>,
    pub unk38: Vec<u32>,
    pub unk48: TagHash,
    pub unk4c: Tag<SOcclusionBounds>,
    pub unk50: Vec<u32>,
    pub unk60: [u32; 4],
    pub bounds: AABB,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80809178 {
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
pub struct Unk8080917b {
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
pub struct Unk808068d4 {
    pub unk0: u32,
    pub unk4: u32,
    pub unk8: u32,
    pub unkc: u32,
    pub entity_model: TagHash,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80808604 {
    pub unk0: [u32; 4],
    pub unk10: Tag<Unk80808724>,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80808606)]
pub struct Unk80808606 {
    pub rotation: glam::Quat,
    pub translation: glam::Vec4,
    pub unk20: glam::Vec4,
    pub unk30: glam::Vec4,
    pub unk40: u32,
    pub shape_index: u32,
    pub unk48: u32,
    pub unk4c: u32,
    pub unk50: [u32; 4],
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80808724 {
    pub file_size: u64,
    pub unk8: Vec<Unk80808606>,
    pub havok_file: TagHash,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0x8080824C)]
pub struct Unk8080824c {
    pub rotation: glam::Quat,
    pub translation: glam::Vec4,
    pub unk20: glam::Vec4,
    pub unk30: glam::Vec4,
    pub unk40: glam::Vec4,
    pub unk50: glam::Vec4,
    pub unk60: Vec<glam::Vec4>,
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
pub struct Unk80808248 {
    pub file_size: u64,
    pub havok_file: TagHash,
    _pad: u32,
    pub unk10: Vec<Unk8080824c>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80808246 {
    pub unk0: [u32; 4],
    pub unk10: Tag<Unk80808248>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80806ac2 {
    pub unk0: [u32; 4],
    pub unk10: Tag<Unk80806ac4>,
    pub array_index: u32,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0xffffffff)]
pub struct Unk80806ac4 {
    pub file_size: u64,
    pub havok_file: TagHash,
    _pad: u32,
    pub unk10: Vec<Unk80806ed8>,
}
#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806ED8)]
pub struct Unk80806ed8 {
    pub rotation: glam::Quat,
    pub translation: glam::Vec4,
    pub unk20: glam::Vec4,
    pub unk30: glam::Vec4,
    pub unk40: glam::Vec4,
    pub unk50: glam::Vec4,

    pub unk60: Vec<glam::Vec4>,
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

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806ABD)]
pub struct Unk80806abd {
    pub file_size: u64,
    pub havok_file: TagHash,
    _pad: u32,
    pub unk10: Vec<Unk80806bb2>,
}

#[derive(Clone, Debug)]
#[tiger_tag(id = 0x80806BB2)]
pub struct Unk80806bb2 {
    pub rotation: glam::Quat,
    pub translation: glam::Vec4,
    pub unk20: glam::Vec4,
    pub unk30: glam::Vec4,
    pub unk40: glam::Vec4,
    pub unk50: glam::Vec4,

    pub unk60: Vec<()>,
    pub unk70: Vec<()>,

    pub unk80: f32,
    pub unk84: [u32; 3],
    pub unk90: [u32; 4],
    pub unka0: [u32; 3],
    pub shape_index: u32,
}
