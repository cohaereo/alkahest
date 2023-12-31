use binrw::BinRead;
use destiny_pkg::TagHash;
use std::io::SeekFrom;

use crate::entity::{ELodCategory, EPrimitiveType};
use crate::map::SOcclusionBounds;
use crate::render::tfx::TfxRenderStage;
use crate::structure::Tag;
use crate::types::Vector2;
use crate::{
    structure::TablePointer,
    types::{Vector3, Vector4},
};

#[derive(BinRead, Debug, Clone)]
pub struct SStaticMesh {
    pub file_size: u64,
    pub unk8: TagHash,
    pub unkc: u32,
    pub materials: TablePointer<TagHash>,
    pub unk20: TablePointer<SStaticMeshOverlay>, // Overlay/transparent meshes
    pub unk30: [u32; 2],
    pub unk38: [f32; 6],
    pub unk50: Vector3, // ? Similar to model_offset, but not quite right...
    pub unk5c: f32,
}

#[derive(BinRead, Debug)]
pub struct SStaticMeshData {
    pub file_size: u64,
    pub mesh_groups: TablePointer<Unk8080719b>,
    pub parts: TablePointer<Unk8080719a>,
    pub buffers: TablePointer<(TagHash, TagHash, TagHash, TagHash)>,

    #[br(seek_before(SeekFrom::Start(0x40)))]
    pub mesh_offset: Vector3,
    pub mesh_scale: f32,
    pub texture_coordinate_scale: f32,
    pub texture_coordinate_offset: Vector2,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk8080719a {
    pub index_start: u32,
    pub index_count: u32,
    pub buffer_index: u8,
    pub unk9: u8,
    pub lod_category: ELodCategory,
    pub primitive_type: EPrimitiveType,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk8080719b {
    pub part_index: u16,
    pub unk2: u8,
    pub unk3: u8,
    pub unk5: u16,
}

#[derive(BinRead, Debug, Clone)]
pub struct SStaticMeshInstances {
    #[br(seek_before(SeekFrom::Current(0x18)))]
    pub occlusion_bounds: Tag<SOcclusionBounds>,

    #[br(seek_before(SeekFrom::Current(0x24)))]
    pub transforms: TablePointer<Unk808071a3>,
    pub unk50: u64,
    pub unk58: [u64; 4],
    pub statics: TablePointer<TagHash>,
    pub instance_groups: TablePointer<SStaticMeshInstanceGroup>,
}

#[derive(BinRead, Debug, Clone)]
pub struct SStaticMeshInstanceGroup {
    pub instance_count: u16,
    pub instance_start: u16,
    pub static_index: u16,
    pub unk6: u16,
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk808071a3 {
    pub rotation: Vector4, // TODO(cohae): Quat type? (alias?)
    pub translation: Vector3,
    pub scale: Vector3,
    pub unk28: u32,
    pub unk2c: u32,
    pub unk30: [u32; 4],
}

#[derive(BinRead, Debug, Clone)]
pub struct SStaticMeshOverlay {
    pub render_stage: TfxRenderStage,
    pub unk1: u8,
    pub lod: ELodCategory,
    pub unk3: i8,
    pub primitive_type: EPrimitiveType,
    pub unk5: u8,
    pub unk6: u16,
    pub index_buffer: TagHash,
    pub vertex_buffer: TagHash,
    pub vertex_buffer2: TagHash,
    pub color_buffer: TagHash,
    pub index_start: u32,
    pub index_count: u32,
    pub material: TagHash,
}
