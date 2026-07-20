use uuid::{Uuid, uuid};

pub mod handle;
pub mod index_buffer;
pub mod manager;
pub mod texture;
pub mod vertex_buffer;

pub use handle::Handle;
pub use manager::AssetManager;

use crate::tfx::technique;

pub trait Asset {
    const ASSET_TYPE: Uuid;
}

impl Asset for texture::Texture {
    const ASSET_TYPE: Uuid = uuid!("a6877fe1-99c2-4015-afd4-b5f8dc5be81d");
}

impl Asset for vertex_buffer::VertexBuffer {
    const ASSET_TYPE: Uuid = uuid!("ca341c44-3706-4a19-8c76-1a756e491485");
}

impl Asset for index_buffer::IndexBuffer {
    const ASSET_TYPE: Uuid = uuid!("d7e5e281-dcec-496d-ade0-ef0079ea29f6");
}

impl Asset for technique::Technique {
    const ASSET_TYPE: Uuid = uuid!("23f52458-9bd3-4bdf-9d73-bebc795ae77e");
}
