use crate::texture::TextureHeader;
use binrw::{BinRead, BinWrite, BinWriterExt};
use ddsfile::{AlphaMode, D3D10ResourceDimension};
use std::io::{Seek, Write};
use std::mem::transmute;

pub fn dump_to_dds<W: Write + Seek>(out: &mut W, tex: &TextureHeader, data: &[u8]) {
    let mut dds = ddsfile::Dds::new_dxgi(ddsfile::NewDxgiParams {
        height: tex.height as u32,
        width: tex.width as u32,
        depth: None,
        format: unsafe { transmute(tex.format) },
        mipmap_levels: None,
        array_layers: Some(tex.array_size as _),
        caps2: None,
        is_cubemap: false, //(tex.array_size % 6) == 0,
        resource_dimension: D3D10ResourceDimension::Texture2D,
        alpha_mode: AlphaMode::Straight,
    })
    .unwrap();

    dds.data = data.to_vec();

    dds.write(out).unwrap();
}
