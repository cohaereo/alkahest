use crate::ecs::Scene;

use crate::render::ConstantBuffer;
use crate::statics::Unk8080966d;
use crate::structure::{ExtendedHash, ExtendedTag, ResourcePointer, TablePointer, Tag};
use crate::types::{ResourceHash, Vector4};
use binrw::BinRead;
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
pub struct Unk8080714f {
    pub file_size: u64,
    #[br(seek_before(SeekFrom::Start(0x10)))]
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
    pub unk0: f32,
    pub unk4: f32,
    pub unk8: f32,
    pub unkc: f32,
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
    pub lights: Vec<Vec4>,
    pub lights_cbuffer: ConstantBuffer<Vec4>,

    pub scene: Scene,
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
