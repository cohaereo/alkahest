use alkahest_data::ExtendedHash;
use destiny_pkg::TagHash;

use crate::{
    gpu::{texture::Texture, GpuContext},
};

pub fn load_texture(gctx: &GpuContext, hash: TagHash) -> anyhow::Result<Texture> {
    Texture::load(&gctx.device, ExtendedHash::Hash32(hash))
}
