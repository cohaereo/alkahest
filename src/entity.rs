use crate::structure::{DeadBeefMarker, TablePointer};
use crate::types::{DecodeFloat, Vector2, Vector3, Vector4};
use anyhow::anyhow;
use binrw::{BinRead, BinReaderExt};
use bytemuck::{Pod, Zeroable};
use destiny_pkg::TagHash;
use glam::{Vec2, Vec3A, Vec4};
use std::cmp::Ordering;
use std::io::{Cursor, Read, Seek, SeekFrom};
use tracing::{debug, warn};

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

#[derive(Default)]
pub struct DecodedVertexBuffer {
    pub positions: Vec<Vector4>,
    pub tex_coords: Vec<Vector4>,
    pub normals: Vec<Vector4>,
    pub tangents: Vec<Vector4>,
    pub colors: Vec<Vector4>,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable)]
pub struct DecodedVertex {
    pub position: [f32; 4],
    pub tex_coord: [f32; 4],
    pub normal: [f32; 4],
    pub tangent: [f32; 4],
    pub color: [f32; 4],
}

pub fn decode_vertices2(
    header: &VertexBufferHeader,
    data: &[u8],
    out: &mut DecodedVertexBuffer,
) -> anyhow::Result<()> {
    let mut cur = Cursor::new(data);

    let count = data.len() / header.stride as usize;
    let mut data = vec![0u8; header.stride as usize];
    for _ in 0..count {
        cur.read_exact(&mut data).unwrap();
        match header.vtype {
            0 => decode_vertex2_0(header.stride, &data, out)?,
            u => anyhow::bail!("Unsupported vertex data type {u}"),
        };
    }

    Ok(())
}

fn decode_vertex2_0(stride: u16, data: &[u8], out: &mut DecodedVertexBuffer) -> anyhow::Result<()> {
    let mut cur = Cursor::new(data);
    match stride {
        8 => {
            let d4: [i16; 4] = cur.read_le().unwrap();
            out.normals.push(d4.into());
        }
        4 => {
            let d2: [i16; 2] = cur.read_le().unwrap();
            out.tex_coords.push(Vector4::new(
                d2[0].decode_float(),
                d2[1].decode_float(),
                1.0,
                1.0,
            ));
        }
        12 => {
            let d2: [i16; 2] = cur.read_le().unwrap();
            out.tex_coords.push(Vector4::new(
                d2[0].decode_float(),
                d2[1].decode_float(),
                1.0,
                1.0,
            ));

            if matches!(stride, 12) {
                let n4: [i16; 4] = cur.read_le().unwrap();
                out.normals.push(n4.into());
            }
        }
        16 => {
            // TODO(cohae): Has more data
            let d2: [i16; 2] = cur.read_le().unwrap();
            out.tex_coords.push(Vector4::new(
                d2[0].decode_float(),
                d2[1].decode_float(),
                1.0,
                1.0,
            ));
            let n4: [i16; 4] = cur.read_le().unwrap();
            out.normals.push(n4.into());
        }
        20 => {
            let d2: [i16; 2] = cur.read_le().unwrap();
            out.tex_coords.push(Vector4::new(
                d2[0].decode_float(),
                d2[1].decode_float(),
                1.0,
                1.0,
            ));
            let n4: [i16; 4] = cur.read_le().unwrap();
            out.normals.push(n4.into());

            let t4: [i16; 4] = cur.read_le().unwrap();
            out.tangents.push(t4.into());
        }
        // TODO(cohae): Format depends on shader
        24 => {
            let d2: [i16; 2] = cur.read_le().unwrap();
            out.tex_coords.push(Vector4::new(
                d2[0].decode_float(),
                d2[1].decode_float(),
                1.0,
                1.0,
            ));
            let n4: [i16; 4] = cur.read_le().unwrap();
            out.normals.push(n4.into());

            // TODO(cohae): Has more data
            // let d4: [i16; 4] = cur.read_le().unwrap();
            // out.tex_coords.push(d4.into());
            // TODO(cohae): Broken normals
            // let n3: [i16; 3] = cur.read_le().unwrap();
            // let t3: [i16; 3] = cur.read_le().unwrap();
            // let n3: Vector3 = n3.into();
            // let t3: Vector3 = t3.into();
            // out.normals.push(Vector4::new(n3.x, n3.y, n3.z, 1.0));
            // out.tangents.push(Vector4::new(t3.x, t3.y, t3.z, 1.0));
            // out.tangents.push(t4.into());
            let u4: [i16; 4] = cur.read_le().unwrap();
            // let n4: [i16; 4] = cur.read_le().unwrap();
            // out.normals.push(n4.into());
            let color: [u8; 4] = cur.read_le().unwrap();
            out.colors.push(color.into());
        }
        u => anyhow::bail!("Unsupported v2_0 stride {u}"),
    };

    Ok(())
}

pub fn decode_vertices(
    header: &VertexBufferHeader,
    data: &[u8],
    out: &mut DecodedVertexBuffer,
) -> anyhow::Result<()> {
    let mut cur = Cursor::new(data);

    let count = data.len() / header.stride as usize;
    let mut data = vec![0u8; header.stride as usize];
    for _ in 0..count {
        cur.read_exact(&mut data).unwrap();
        match header.vtype {
            0 => decode_vertex0(header.stride, &data, out)?,
            u => anyhow::bail!("Unsupported vertex data type {u}"),
        };
    }

    Ok(())
}

fn decode_vertex0(stride: u16, data: &[u8], out: &mut DecodedVertexBuffer) -> anyhow::Result<()> {
    let mut c = Cursor::new(data);
    match stride {
        8 => {
            let d4: [i16; 4] = c.read_le().unwrap();
            out.positions.push(d4.into());
        }
        12 => {
            let pos: [i16; 4] = c.read_le().unwrap();
            let uv: [i16; 2] = c.read_le().unwrap();
            out.positions.push(pos.into());
            out.tex_coords.push(Vector4::new(
                uv[0].decode_float(),
                uv[1].decode_float(),
                1.0,
                1.0,
            ));
        }
        28 | 32 => {
            let pos: [i16; 4] = c.read_le().unwrap();
            let uv: [i16; 2] = c.read_le().unwrap();
            let normal: [i16; 4] = c.read_le().unwrap();
            let tangent: [i16; 4] = c.read_le().unwrap();
            out.positions.push(pos.into());
            out.tex_coords.push(Vector4::new(
                uv[0].decode_float(),
                uv[1].decode_float(),
                1.0,
                1.0,
            ));
            out.normals.push(normal.into());
            out.tangents.push(tangent.into());

            if stride == 32 {
                let color: [u8; 4] = c.read_le().unwrap();
                out.colors.push(color.into());
            }
        }
        16 | 40 => {
            // TODO(cohae): More data to be discovered
            let d4: [i16; 4] = c.read_le().unwrap();
            out.positions.push(d4.into());
        }
        20 => {
            warn!("Stubbed V0 stride 20");
            // TODO(cohae): Stubbed
            out.positions.push(Vector4::default());
        }
        24 => {
            out.positions.push(c.read_le().unwrap());
            // TODO(cohae): Almost definitely wrong
            out.tex_coords.push(c.read_le().unwrap());
        }
        48 => {
            out.positions.push(c.read_le().unwrap());
            // TODO(cohae): Almost definitely wrong
            out.normals.push(c.read_le().unwrap());
            out.tangents.push(c.read_le().unwrap());
        }
        u => anyhow::bail!("Unsupported v0 stride {u}"),
    };

    Ok(())
}
