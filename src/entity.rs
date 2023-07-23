use crate::structure::{DeadBeefMarker, TablePointer};
use crate::types::{Vector2, Vector4};
use binrw::{BinRead, BinReaderExt};
use destiny_pkg::TagHash;
use std::cmp::Ordering;
use std::io::{Cursor, Read, SeekFrom};

#[derive(BinRead, Debug)]
pub struct Unk808073a5 {
    pub file_size: u64,
    pub unk0: u64,
    pub unk1: TablePointer<Unk80807378>,
    #[br(seek_before(SeekFrom::Current(0x30)))]
    pub model_scale: Vector4,
    pub model_translation: Vector4,
    pub texcoord_scale: Vector2,
    pub texcoord_translation: Vector2,
}

#[derive(BinRead, Debug)]
pub struct Unk80807378 {
    pub position_buffer: TagHash,
    pub secondary_vertex_buffer: TagHash,
    pub buffer2: TagHash,
    pub buffer3: TagHash,
    pub index_buffer: TagHash,
    pub unk0: u32,
    pub unk1: TablePointer<Unk8080737e>,
    pub unk2: [u16; 48],
}

#[derive(BinRead, Debug, Clone)]
pub struct Unk8080737e {
    pub material: TagHash,
    pub unk4: u16,
    pub primitive_type: EPrimitiveType,
    pub unk7: u8,
    pub index_start: u32,
    pub index_count: u32,
    pub unk10: u32,
    pub unk14: u32,
    pub unk18: u8,
    pub unk19: u8,
    pub unk1a: u8,
    pub lod_category: ELodCategory,
    pub unk1c: u32,
}

#[derive(BinRead, Debug, PartialEq, Copy, Clone)]
#[br(repr(u8))]
pub enum EPrimitiveType {
    Triangles = 3,
    TriangleStrip = 5,
}

#[allow(non_camel_case_types, clippy::derive_ord_xor_partial_ord)]
#[derive(BinRead, Debug, PartialEq, Eq, Ord, Copy, Clone)]
#[br(repr(u8))]
pub enum ELodCategory {
    Lod_0_0 = 0,     // main geometry lod0
    Lod_0_1 = 1,     // grip/stock lod0
    Lod_0_2 = 2,     // stickers lod0
    Lod_0_3 = 3,     // internal geom lod0
    Lod_1_0 = 4,     // low poly geom lod1
    Lod_2_0 = 7,     // low poly geom lod2
    Lod_2_1 = 8,     // grip/stock/scope lod2
    Lod_3_0 = 9,     // low poly geom lod3
    Lod_Detail = 10, // detail lod0
}

impl PartialOrd for ELodCategory {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.remap_order().partial_cmp(&other.remap_order())
    }
}

impl ELodCategory {
    // Remap the order of variants for sorting purposes, starting with the lowest level
    fn remap_order(&self) -> u8 {
        match self {
            ELodCategory::Lod_Detail => 10,
            ELodCategory::Lod_0_0 => 9,
            ELodCategory::Lod_0_1 => 8,
            ELodCategory::Lod_0_2 => 7,
            ELodCategory::Lod_0_3 => 4,
            ELodCategory::Lod_1_0 => 3,
            ELodCategory::Lod_2_0 => 2,
            ELodCategory::Lod_2_1 => 1,
            ELodCategory::Lod_3_0 => 0,
        }
    }

    pub fn is_highest_detail(&self) -> bool {
        matches!(
            self,
            ELodCategory::Lod_0_0
                | ELodCategory::Lod_0_1
                | ELodCategory::Lod_0_2
                | ELodCategory::Lod_0_3
                | ELodCategory::Lod_Detail
        )
    }

    pub fn is_lowest_detail(&self) -> bool {
        matches!(self, ELodCategory::Lod_3_0)
    }
}

#[derive(BinRead, Debug)]
pub struct VertexBufferHeader {
    pub data_size: u32,
    pub stride: u16,
    pub vtype: u16,
    pub deadbeef: DeadBeefMarker,
}

#[derive(BinRead, Debug)]
pub struct IndexBufferHeader {
    pub unk0: i8,
    #[br(map(|v: u8| v != 0))]
    pub is_32bit: bool,
    pub unk1: u16, // Probably padding
    pub zero: u32,
    pub data_size: u64,
    pub deadbeef: DeadBeefMarker,
    pub zero1: u32,
}

pub fn decode_vertices2(header: &VertexBufferHeader, data: &[u8]) -> Vec<Vector2> {
    let mut cur = Cursor::new(data);
    let mut vertices = vec![];

    let count = data.len() / header.stride as usize;
    let mut data = vec![0u8; header.stride as usize];
    for _ in 0..count {
        cur.read_exact(&mut data).unwrap();
        let v = match header.vtype {
            0 => decode_vertex2_0(header.stride, &data),
            u => panic!("Unsupported vertex data type {u}"),
        };

        vertices.push(v);
    }

    vertices
}

fn decode_vertex2_0(stride: u16, data: &[u8]) -> Vector2 {
    match stride {
        0x4 => {
            // TODO(cohae): Has more data
            let d: &[i16] = bytemuck::cast_slice(data);
            let d2: [i16; 2] = d.try_into().unwrap();

            Vector2::from(d2)
        }
        0x8 => {
            // TODO(cohae): Stubbed for now
            Vector2::default()
        }
        0x10 => {
            // TODO(cohae): Has more data
            let d: &[i16] = bytemuck::cast_slice(data);
            let d2: [i16; 2] = d[4..6].try_into().unwrap();

            Vector2::from(d2)
        }
        0xc | 0x14 | 0x18 | 0x1c => {
            // TODO(cohae): Has more data
            let d: &[i16] = bytemuck::cast_slice(data);
            let d2: [i16; 2] = d[0..2].try_into().unwrap();

            Vector2::from(d2)
        }
        u => panic!("Unsupported v2_0 stride 0x{u:x}"),
    }
}

pub fn decode_vertices(header: &VertexBufferHeader, data: &[u8]) -> Vec<Vector4> {
    let mut cur = Cursor::new(data);
    let mut vertices = vec![];

    let count = data.len() / header.stride as usize;
    let mut data = vec![0u8; header.stride as usize];
    for _ in 0..count {
        cur.read_exact(&mut data).unwrap();
        let v = match header.vtype {
            0 => decode_vertex0(header.stride, &data),
            u => panic!("Unsupported vertex data type {u}"),
        };

        vertices.push(v);
    }

    vertices
}

fn decode_vertex0(stride: u16, data: &[u8]) -> Vector4 {
    let mut c = Cursor::new(data);
    match stride {
        0x8 => {
            let d4: [i16; 4] = c.read_le().unwrap();
            (Vector4::from(d4))
        }
        0xc | 0x10 | 0x1c | 0x20 | 0x28 => {
            // TODO(cohae): More data to be discovered
            let d4: [i16; 4] = c.read_le().unwrap();
            (Vector4::from(d4))
        }
        0x14 => {
            // TODO(cohae): Stubbed
            (Vector4::default())
        }
        0x18 | 0x30 => {
            let d_pos: [f32; 4] = c.read_le().unwrap();
            (Vector4::from(d_pos))
        }
        u => panic!("Unsupported v0 stride 0x{u:x}"),
    }
}
