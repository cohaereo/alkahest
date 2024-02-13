use std::{cmp::Ordering, mem::transmute};

use tiger_parse::TigerReadable;

#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum EPrimitiveType {
    Triangles = 3,
    TriangleStrip = 5,
}

impl TigerReadable for EPrimitiveType {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        Ok(unsafe { transmute(u8::read_ds_endian(reader, endian)?) })
    }

    const ZEROCOPY: bool = true;
    const SIZE: usize = 1;
}

// impl EPrimitiveType {
//     pub fn to_dx(self) -> D3D_PRIMITIVE_TOPOLOGY {
//         match self {
//             EPrimitiveType::Triangles => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
//             EPrimitiveType::TriangleStrip => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
//         }
//     }
// }

#[allow(non_camel_case_types, clippy::derive_ord_xor_partial_ord)]
#[derive(Debug, PartialEq, Eq, Ord, Copy, Clone)]
#[repr(u8)]
pub enum ELodCategory {
    /// main geometry lod0
    Lod_0_0 = 0,
    /// grip/stock lod0
    Lod_0_1 = 1,
    /// stickers lod0
    Lod_0_2 = 2,
    /// internal geom lod0
    Lod_0_3 = 3,
    /// low poly geom lod1
    Lod_1_0 = 4,
    /// low poly geom lod2
    Lod_2_0 = 7,
    /// grip/stock/scope lod2
    Lod_2_1 = 8,
    /// low poly geom lod3
    Lod_3_0 = 9,
    /// detail lod0
    Lod_Detail = 10,
}

impl TigerReadable for ELodCategory {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        Ok(unsafe { transmute(u8::read_ds_endian(reader, endian)?) })
    }

    const ZEROCOPY: bool = true;
    const SIZE: usize = 1;
}

impl PartialOrd for ELodCategory {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.remap_order().cmp(&other.remap_order()))
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
}
