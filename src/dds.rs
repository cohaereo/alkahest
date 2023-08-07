use crate::texture::TextureHeader;
use binrw::BinWrite;
use ddsfile::{AlphaMode, D3D10ResourceDimension};
use std::io::{Seek, Write};
use std::mem::transmute;

pub fn dump_to_dds<W: Write + Seek>(out: &mut W, tex: &TextureHeader, data: &[u8]) {
    let mut dds = ddsfile::Dds::new_dxgi(ddsfile::NewDxgiParams {
        height: tex.height as u32,
        width: tex.width as u32,
        depth: Some(tex.depth as u32),
        format: unsafe { transmute(tex.format) },
        mipmap_levels: None,
        array_layers: Some(tex.array_size as _),
        caps2: None,
        is_cubemap: (tex.array_size % 6) == 0,
        resource_dimension: if tex.depth == 1 {
            D3D10ResourceDimension::Texture2D
        } else {
            D3D10ResourceDimension::Texture3D
        },
        alpha_mode: AlphaMode::Straight,
    })
    .unwrap();

    dds.data = data.to_vec();

    dds.write(out).unwrap();
}
