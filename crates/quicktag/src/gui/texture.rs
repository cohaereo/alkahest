use crate::gui::dxgi::DxgiFormat;
use crate::packages::package_manager;
use anyhow::Context;
use binrw::BinRead;
use destiny_pkg::TagHash;
use eframe::egui_wgpu::RenderState;
use eframe::wgpu;
use eframe::wgpu::util::DeviceExt;
use eframe::wgpu::TextureDimension;
use std::io::SeekFrom;

#[derive(BinRead)]
pub struct CafeMarker(#[br(assert(self_0 == 0xcafe))] u16);

#[derive(BinRead)]
pub struct TextureHeader {
    pub data_size: u32,
    pub format: DxgiFormat,
    pub _unk8: u32,

    #[br(seek_before = SeekFrom::Start(0x20))]
    pub cafe: CafeMarker,

    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub array_size: u16,

    pub unk2a: u16,
    pub unk2c: u8,
    pub mip_count: u8,
    pub unk2e: [u8; 10],
    pub unk38: u32,

    #[br(seek_before = SeekFrom::Start(0x3c))]
    #[br(map(|v: u32| (v != u32::MAX).then_some(TagHash(v))))]
    pub large_buffer: Option<TagHash>,
}

pub struct Texture {
    pub view: wgpu::TextureView,
    pub handle: wgpu::Texture,
    pub format: DxgiFormat,
    pub aspect_ratio: f32,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

impl Texture {
    pub fn load_data(
        hash: TagHash,
        load_full_mip: bool,
    ) -> anyhow::Result<(TextureHeader, Vec<u8>)> {
        let texture_header_ref = package_manager()
            .get_entry(hash)
            .context("Texture header entry not found")?
            .reference;

        let texture: TextureHeader = package_manager().read_tag_struct(hash)?;
        let mut texture_data = if let Some(t) = texture.large_buffer {
            package_manager()
                .read_tag(t)
                .context("Failed to read texture data")?
        } else {
            package_manager()
                .read_tag(texture_header_ref)
                .context("Failed to read texture data")?
                .to_vec()
        };

        if load_full_mip && texture.large_buffer.is_some() {
            let ab = package_manager()
                .read_tag(texture_header_ref)
                .context("Failed to read large texture buffer")?
                .to_vec();

            texture_data.extend(ab);
        }

        Ok((texture, texture_data))
    }

    pub fn load(rs: &RenderState, hash: TagHash) -> anyhow::Result<Texture> {
        let (texture, texture_data) = Self::load_data(hash, true)?;

        let handle = rs.device.create_texture_with_data(
            &rs.queue,
            &wgpu::TextureDescriptor {
                label: Some(&*format!("Texture {hash}")),
                size: wgpu::Extent3d {
                    width: texture.width as _,
                    height: texture.height as _,
                    depth_or_array_layers: if texture.depth == 1 {
                        texture.array_size as _
                    } else {
                        // texture.depth as _
                        1
                    },
                },
                mip_level_count: texture.mip_count as u32,
                sample_count: 1,
                dimension: TextureDimension::D2,
                // dimension: if texture.depth == 1 {
                //     TextureDimension::D2
                // } else {
                //     TextureDimension::D3
                // },
                format: texture.format.to_wgpu()?,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[texture.format.to_wgpu()?],
            },
            &texture_data,
        );

        let view = handle.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: None,
            dimension: None,
            aspect: Default::default(),
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        Ok(Texture {
            view,
            handle,
            format: texture.format,
            aspect_ratio: texture.width as f32 / texture.height as f32,
            width: texture.width as u32,
            height: texture.height as u32,
            depth: texture.depth as u32,
        })
    }
}
