use crate::ecs::Scene;

use crate::render::ConstantBuffer;
use crate::statics::Unk8080966d;
use crate::structure::{ExtendedHash, ExtendedTag, RelPointer, ResourcePointer, TablePointer, Tag};
use crate::types::{Matrix4, ResourceHash, Vector4, AABB};
use binrw::{BinRead, NullString};
use destiny_pkg::{TagHash, TagHash64};
use glam::Vec4;

use std::fmt::Debug;
use std::io::SeekFrom;

#[derive(BinRead, Debug)]
pub struct SBubbleParent {
    pub file_size: u64,
    // 808091e0
    pub child_map: Tag<SBubbleDefinition>,
    pub unkc: u32,

    pub unk10: u64,
    pub map_name: ResourceHash,

    #[br(seek_before(SeekFrom::Start(0x40)))]
    pub unk40: TablePointer<Unk80809644>,
}

#[derive(BinRead, Debug)]
pub struct Unk80809644 {
    pub unk0: u32,
    pub unk4: u32,
    pub unk8: u32,
    pub unkc: u32, // 8080964e
}

// D2Class_01878080
#[derive(BinRead, Debug)]
pub struct SBubbleDefinition {
    pub file_size: u64,
    pub map_resources: TablePointer<ExtendedTag<SMapContainer>>,
}

// D2Class_07878080
#[derive(BinRead, Debug)]
pub struct SMapContainer {
    pub file_size: u64,
    #[br(seek_before(SeekFrom::Start(0x28)))]
    pub data_tables: TablePointer<Tag<SMapDataTable>>,
}

// D2Class_83988080
#[derive(BinRead, Debug)]
pub struct SMapDataTable {
    pub file_size: u64,
    pub data_entries: TablePointer<Unk808099d8>,
}

// D2Class_85988080
#[derive(BinRead, Clone, Debug)]
pub struct Unk808099d8 {
    pub rotation: Vector4,    // 0x0
    pub translation: Vector4, // 0x10
    pub entity_old: TagHash,  // 0x20
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

#[derive(BinRead, Debug)]
pub struct Unk80806ef4 {
    pub unk0: u64,
    pub placement_group: Tag<Unk8080966d>,
    pub unkc: [u32; 7],
}

/// Terrain
#[derive(BinRead, Debug)]
pub struct STerrain {
    pub file_size: u64,
    pub unk8: u64,

    pub unk10: Vector4,
    pub unk20: Vector4,
    pub unk30: Vector4,

    #[br(seek_before(SeekFrom::Start(0x50)))]
    pub mesh_groups: TablePointer<Unk80807154>,

    pub vertex_buffer: TagHash,
    pub vertex_buffer2: TagHash,
    pub indices: TagHash,
    pub material1: TagHash,
    pub material2: TagHash,

    #[br(seek_before(SeekFrom::Start(0x78)))]
    pub mesh_parts: TablePointer<Unk80807152>,
}

#[derive(BinRead, Debug)]
pub struct Unk80807154 {
    pub unk0: Vector4,
    pub unk10: f32,
    pub unk14: f32,
    pub unk18: f32,
    pub unk1c: u32,
    pub unk20: Vector4,
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

#[derive(BinRead, Debug)]
pub struct Unk80807152 {
    pub material: TagHash,
    pub index_start: u32,
    pub index_count: u16,
    pub group_index: u8,
    pub detail_level: u8,
}

pub struct MapData {
    pub hash: TagHash,
    pub name: String,
    pub placement_groups: Vec<Tag<Unk8080966d>>,
    // pub resource_points: Vec<(ResourcePoint, ConstantBuffer<ScopeRigidModel>)>,
    pub terrains: Vec<TagHash>,
    pub lights: Vec<SimpleLight>,
    pub lights_cbuffer: ConstantBuffer<SimpleLight>,

    pub scene: Scene,
}

#[derive(Clone)]
pub struct SimpleLight {
    pub pos: Vec4,
    pub attenuation: Vec4,
}

pub struct MapDataList {
    pub current_map: usize, // TODO(cohae): Shouldn't be here
    pub maps: Vec<(TagHash, Option<TagHash64>, MapData)>,
}

impl MapDataList {
    pub fn current_map(&self) -> Option<&(TagHash, Option<TagHash64>, MapData)> {
        if self.maps.is_empty() {
            None
        } else {
            self.maps.get(self.current_map % self.maps.len())
        }
    }
}

/// Terrain resource
#[derive(BinRead, Debug, Clone)]
pub struct Unk8080714b {
    #[br(seek_before(SeekFrom::Current(0x10)))]
    pub unk10: u16,
    pub unk12: u16,
    pub unk14: ResourceHash,
    pub terrain: TagHash,
    pub terrain_bounds: TagHash,
}

/// Cubemap volume resource
#[derive(BinRead, Debug, Clone)]
pub struct Unk80806b7f {
    #[br(seek_before(SeekFrom::Current(0x20)))]
    pub cubemap_extents: Vector4,
    /// Represents the visual center of the cubemap
    pub cubemap_center: Vector4,
    pub unk40: f32,
    pub unk44: [u32; 3],
    pub unk50: Vector4,
    pub unk60: Vector4,

    pub unk70: [u32; 20],

    // Transform matrices?
    pub unkc0: [Vector4; 4],
    pub unk100: [Vector4; 4],

    pub unk140: [u32; 28],

    pub cubemap_name: RelPointer<NullString>,
    pub cubemap_texture: TagHash,
    pub unk1bc: u32,
    pub unk1c0: TagHash,
    pub unk1c4: [u32; 7],
}

/// Decal collection resource
#[derive(BinRead, Debug, Clone)]
pub struct Unk80806e68 {
    pub file_size: u64,
    pub instances: TablePointer<Unk80806e6c>,
    pub transforms: TablePointer<Vector4>, // 80806e6d
    pub instance_points: TagHash,
    pub unk_vertex_colors: TagHash,

    pub unk30: [u32; 2],
    pub occlusion_bounds: Tag<SOcclusionBounds>,
    _pad3c: u32,
    pub bounds: AABB,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806e6c {
    pub material: TagHash,
    pub start: u16,
    pub count: u16,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806df3 {
    pub file_size: u64,
    pub unk8: TablePointer<Unk80806dec>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806dec {
    pub material: TagHash,
    pub index_buffer: TagHash,
    pub vertex_buffer: TagHash,
    pub unkc: u32,
    pub unk10: [u32; 4],

    pub translation: Vector4,

    pub unk30: Vector4,
    pub unk40: Vector4,
    pub unk50: Vector4,
}

// Unknown resource (some kind of octree?)
#[derive(BinRead, Debug, Clone)]
pub struct Unk80807268 {
    pub file_size: u64,
    /// Vertex buffer
    pub unk8: TagHash,
    pub unkc: u32,
    pub unk10: TablePointer<Unk8080726a>,
    pub unk20: [u32; 6],
    /// Vertex buffer
    pub unk38: TagHash,
    pub unk3c: u32,
    pub unk40: TablePointer<Unk8080726a>,
    pub unk50: TablePointer<Unk8080726d>,
    pub unk60: TablePointer<u16>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk8080726a {
    pub unk0: [u32; 4],
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk8080726d {
    pub unk0: Vector4,
    pub unk10: Vector4,
    pub unk20: Vector4,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80809162 {
    pub file_size: u64,
    pub unk8: TablePointer<Unk80809164>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80809164 {
    pub unk0: Vector4,
    pub unk10: Vector4,
    pub unk20: [u32; 4],
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80809802 {
    pub file_size: u64,
    pub unk8: TagHash,
    pub unkc: TagHash,
    pub unk10: u32,
    pub unk14: TagHash,
    pub unk18: TagHash,
    pub unk1c: u32,
    pub streams: TablePointer<TagHash>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806aa7 {
    pub file_size: u64,
    pub unk8: TablePointer<Unk80806aa9>,
    pub unk18: TablePointer<Unk808093b3>,
    pub unk28: TablePointer<u32>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806aa9 {
    /// Transformation matrix
    pub transform: [Vector4; 4],

    /// Same as the bounding box from the Unk808093b3 array
    pub bounds: AABB,

    pub unk60: Tag<Unk80806aae>,
    pub unk64: f32,
    pub unk68: u32,
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

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806aae {
    pub file_size: u64,
    pub entity_model: TagHash,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk808093b3 {
    pub bb: AABB,
    pub unk20: [u32; 4],
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806c65 {
    pub file_size: u64,
    pub unk8: u64,
    pub bounds: AABB,
    pub unk30: TablePointer<Unk80806c70>,
    pub unk40: TablePointer<Unk80809f4f>,
    pub light_count: u32,
    pub unk54: u32,
    pub occlusion_bounds: Tag<SOcclusionBounds>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806c70 {
    pub unk0: Vector4,
    pub unk10: Vector4,
    pub unk20: Vector4,
    pub unk30: Vector4,
    pub unk40: [u32; 4],
    pub unk50: Vector4,
    pub unk60: Matrix4,
    pub unka0: u32,
    pub unka4: u32,
    pub unka8: u32,
    pub unkac: f32,
    pub unkb0: f32,
    pub unkb4: f32,
    pub unkb8: f32,
    pub unkbc: f32,

    pub technique_unkc0: TagHash,
    pub technique_unkc4: TagHash,
    pub compute_technique_unkc8: TagHash,
    pub unkcc: TagHash,
    pub unkd0: TagHash,
    pub unkd4: [u32; 7],
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80809f4f {
    pub rotation: Vector4,
    pub translation: Vector4,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80808cb7 {
    pub file_size: u64,
    pub unk8: TablePointer<Unk80808cb9>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80808cb9 {
    pub rotation: Vector4,
    pub translation: Vector4,
    pub unk20: [u32; 4],
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk808085c2 {
    pub file_size: u64,
    pub unk8: TablePointer<Unk808085c4>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk808085c4 {
    pub unk0: [u32; 4],
    pub unk10: [u32; 4],
    pub translation: Vector4,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806d19 {
    pub file_size: u64,
    pub unk8: TagHash,
    pub unkc: u32, // Padding
    pub unk10: TablePointer<()>,
    pub unk20: TagHash,
    pub unk24: u32, // Padding
    pub unk28: TablePointer<()>,
    pub unk38: TagHash,
    pub unk3c: u32, // Padding
    pub unk40: TablePointer<()>,
    pub unk50: TablePointer<Unk80806d4f>,
    pub unk60: TablePointer<()>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80806d4f {
    pub translation: Vector4,
    pub unk10: [u32; 4],
    pub unk20: [u32; 4],
}

// #[derive(BinRead, Debug, Clone)]
// pub struct Unk808066a2 {
//     pub file_size: u64,
//     pub unk8: TablePointer<()>,
//     pub unk18: TablePointer<()>,
//     /// Havok file
//     pub unk28: TagHash,
// }

#[derive(BinRead, Debug)]
pub struct Unk80806c98 {
    pub file_size: u64,
    pub unk8: TablePointer<TagHash>,
    pub unk18: TablePointer<u32>,
    pub unk28: TablePointer<u32>,
    pub unk38: TablePointer<u32>,
    pub unk48: TagHash,
    pub unk4c: Tag<SOcclusionBounds>,
    pub unk50: TablePointer<u32>,
    pub unk60: [u32; 4],
    pub bounds: AABB,
}

/// B1938080
#[derive(BinRead, Debug, Clone)]
pub struct SOcclusionBounds {
    pub file_size: u64,
    pub bounds: TablePointer<SMeshInstanceOcclusionBounds>,
}

// B3938080
#[derive(BinRead, Debug, Clone)]
pub struct SMeshInstanceOcclusionBounds {
    pub bb: AABB,
    pub unk20: [u32; 4],
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80809178 {
    // Points to havok pre-tag
    pub unk0: RelPointer<Unk80809121>,

    pub unk8: u32,
    pub unkc: u32,
    pub area_name: ResourceHash,
    pub unk14: ResourceHash,
    pub unk18: ResourceHash,

    // Absolute offset to havok pre-tag??
    pub unk1c: u64,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk8080917b {
    // Points to havok pre-tag
    pub unk0: RelPointer<Unk80809121>,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk80809121 {
    pub unk0: [u32; 4],
    pub havok_file: TagHash,
    pub unk14: u32,
    pub unk18: u32,
}

#[derive(BinRead, Clone)]
pub struct Unk808068d4 {
    pub unk0: u32,
    pub unk4: u32,
    pub unk8: u32,
    pub unkc: u32,
    pub entity_model: TagHash,
}
