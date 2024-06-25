use std::io::Cursor;

use alkahest_pm::package_manager;
use anyhow::Context;
use destiny_havok::shape_collection;
use destiny_pkg::TagHash;

pub struct HavokShape {
    pub shape: shape_collection::Shape,
}

// impl HavokShape {
//     pub fn load_shapes(tag: TagHash, shape_index: u32) -> anyhow::Result<Self> {
//         let havok_data = package_manager()
//             .read_tag(tag)
//             .context("Failed to read havok tag")?;
//
//         let mut cur = Cursor::new(&havok_data);
//         let shapes = shape_collection::read_shape_collection(&mut cur)
//             .context("Failed to read shape collection")?;
//
//         Ok(Self { shape })
//     }
// }
