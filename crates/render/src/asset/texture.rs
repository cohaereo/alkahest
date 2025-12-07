use std::{
    io::Cursor,
    sync::atomic::{AtomicBool, Ordering},
};

use alkahest_data::{
    tag::WideHash,
    tfx::{
        texture::{DxgiFormat, STextureHeader},
        ShaderStage,
    },
};
use anyhow::{ensure, Context};
use d3d11::{
    dxgi, BindFlags, DeviceChild, ResourceMiscFlags, SamplerDesc, Texture2dDesc, Texture3dDesc,
    D3D11_SUBRESOURCE_DATA,
};
use ddsfile::Dds;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{package_manager, TagHash};
use tracing::{debug_span, error};

use crate::{
    gpu::command_list::{CommandList, ContextExt},
    util::d3d::calc_dx_subresource,
    Gpu,
};

pub static LOW_RES: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
pub enum TextureHandle {
    Texture2D(d3d11::Texture2D),
    TextureCube(d3d11::Texture2D),
    Texture3D(d3d11::Texture3D),
}

#[derive(Clone)]
pub struct Texture {
    pub view: d3d11::ShaderResourceView,
    pub handle: TextureHandle,
    pub format: DxgiFormat,

    pub width: u32,
    pub height: u32,
}

impl Texture {
    #[profiling::function]
    pub fn load_data(
        hash: WideHash,
        load_full_mip: bool,
    ) -> anyhow::Result<(STextureHeader, Vec<u8>)> {
        let texture_header_ref = package_manager()
            .get_entry(hash)
            .with_context(|| format!("Texture header entry for {hash} not found"))?
            .reference;

        let texture: STextureHeader = package_manager().read_tag_struct(hash)?;
        let mut texture_data = if texture.large_buffer.is_some() {
            package_manager()
                .read_tag(texture.large_buffer)
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

    #[profiling::function]
    pub fn load<H: Into<WideHash>>(device: &d3d11::Device, hash: H) -> anyhow::Result<Texture> {
        let hash = hash.into();
        let _span = debug_span!("Load texture", ?hash).entered();
        let (texture, texture_data) = Self::load_data(hash, true)?;

        let (tex, view) = unsafe {
            if texture.depth > 1 {
                let (pitch, slice_pitch) = texture
                    .format
                    .calculate_pitch(texture.width as _, texture.height as _);
                let initial_data = D3D11_SUBRESOURCE_DATA {
                    pSysMem: texture_data.as_ptr() as _,
                    SysMemPitch: pitch as _,
                    SysMemSlicePitch: slice_pitch as _,
                };

                let _span_load = debug_span!("Load texture3d").entered();
                // let mut tex = None;
                // device
                //     .CreateTexture3D(
                //         &D3D11_TEXTURE3D_DESC {
                //             Width: texture.width as _,
                //             Height: texture.height as _,
                //             Depth: texture.depth as _,
                //             MipLevels: 1,
                //             Format: dxgi_to_win(texture.format),
                //             Usage: D3D11_USAGE_DEFAULT,
                //             BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                //             CPUAccessFlags: Default::default(),
                //             MiscFlags: Default::default(),
                //         },
                //         Some([initial_data].as_ptr()),
                //         Some(&mut tex),
                //     )
                //     .context("Failed to create 3D texture")?;

                // let tex = tex.unwrap();

                let tex = device.create_texture3d(
                    &Texture3dDesc::builder()
                        .width(texture.width as _)
                        .height(texture.height as _)
                        .depth(texture.depth as _)
                        .mip_levels(1)
                        .format(texture.format.into())
                        .usage(d3d11::Usage::Default)
                        .bind_flags(BindFlags::SHADER_RESOURCE)
                        .build(),
                    Some(&[initial_data]),
                )?;

                tex.set_debug_name(format!("Texture3D {hash}"));

                let view = device.create_shader_resource_view(&tex, None)?;

                (TextureHandle::Texture3D(tex), view)
            } else if texture.array_size > 1 {
                let texture_data = Box::new(texture_data);

                let mut initial_data = vec![
                    Default::default();
                    texture.mip_count as usize * texture.array_size as usize
                ];

                let mut offset = 0;
                let mip_count = texture.mip_count as usize;
                for i in 0..mip_count {
                    for e in 0..texture.array_size as usize {
                        let width = texture.width >> i;
                        let height = texture.height >> i;
                        let (pitch, slice_pitch) =
                            texture.format.calculate_pitch(width as u32, height as u32);

                        initial_data[calc_dx_subresource(i, e, mip_count)] =
                            D3D11_SUBRESOURCE_DATA {
                                pSysMem: texture_data.as_ptr().add(offset) as _,
                                SysMemPitch: pitch,
                                SysMemSlicePitch: 0,
                            };
                        offset += slice_pitch as usize;
                    }
                }

                let _span_load = debug_span!("Load texturecube").entered();
                let tex = device.create_texture2d(
                    &Texture2dDesc::builder()
                        .width(texture.width as _)
                        .height(texture.height as _)
                        .mip_levels(mip_count as _)
                        .array_size(texture.array_size as _)
                        .format(texture.format.into())
                        .bind_flags(BindFlags::SHADER_RESOURCE)
                        .misc_flags(ResourceMiscFlags::TEXTURECUBE)
                        .build(),
                    Some(&initial_data),
                )?;

                tex.set_debug_name(format!("TextureCube {hash}"));

                let view = device.create_shader_resource_view(&tex, None)?;

                (TextureHandle::TextureCube(tex), view)
            } else {
                // TODO(cohae): mips break sometimes when using the full value from the header when there's no large buffer, why?
                let mut mipcount_fixed = if texture.large_buffer.is_some() {
                    texture.mip_count
                } else {
                    1
                };

                let mut initial_data = vec![];
                let mut offset = 0;
                for i in 0..mipcount_fixed {
                    let width: u16 = texture.width >> i;
                    let height = texture.height >> i;
                    let (pitch, slice_pitch) =
                        texture.format.calculate_pitch(width as u32, height as u32);

                    if pitch == 0 {
                        mipcount_fixed = i;
                        break;
                    }

                    initial_data.push(D3D11_SUBRESOURCE_DATA {
                        pSysMem: texture_data.as_ptr().add(offset) as _,
                        SysMemPitch: pitch,
                        SysMemSlicePitch: 0,
                    });
                    offset += slice_pitch as usize;
                }

                let mut verylowres_mip = 0;
                if LOW_RES.load(Ordering::Relaxed) {
                    // Remove everything but mips under 4x4
                    let mut new_data = vec![];
                    for i in 0..mipcount_fixed {
                        let width: u16 = texture.width >> i;
                        let height = texture.height >> i;
                        if width <= 8 || height <= 8 {
                            if verylowres_mip == 0 {
                                verylowres_mip = i;
                            }

                            new_data.push(initial_data[i as usize]);
                        }
                    }

                    if !new_data.is_empty() {
                        initial_data = new_data;
                    }
                }

                if mipcount_fixed < 1 {
                    error!(
                        "Invalid mipcount for texture {hash:?} (width={}, height={}, mips={})",
                        texture.width, texture.height, texture.mip_count
                    );
                }

                let _span_load = debug_span!("Load texture2d").entered();
                let tex = device.create_texture2d(
                    &Texture2dDesc::builder()
                        .width((texture.width >> verylowres_mip) as _)
                        .height((texture.height >> verylowres_mip) as _)
                        .mip_levels(initial_data.len() as _)
                        .array_size(1)
                        .format(texture.format.into())
                        .bind_flags(BindFlags::SHADER_RESOURCE)
                        .build(),
                    Some(&initial_data),
                )?;

                tex.set_debug_name(format!("Texture2D {hash}"));

                let view = device.create_shader_resource_view(&tex, None)?;

                (TextureHandle::Texture2D(tex), view)
            }
        };

        Ok(Texture {
            handle: tex,
            view,
            format: texture.format,
            width: texture.width as u32,
            height: texture.height as u32,
        })
    }

    pub fn load_2d_raw(
        device: &d3d11::Device,
        width: u32,
        height: u32,
        data: &[u8],
        format: impl Into<DxgiFormat>,
        name: Option<&str>,
        uav: bool,
    ) -> anyhow::Result<Texture> {
        let format = format.into();
        let desc = Texture2dDesc::builder()
            .width(width)
            .height(height)
            .mip_levels(1)
            .array_size(1)
            .format(format.into())
            .bind_flags(
                BindFlags::SHADER_RESOURCE
                    | if uav {
                        BindFlags::UNORDERED_ACCESS
                    } else {
                        BindFlags::empty()
                    },
            )
            .usage(d3d11::Usage::Default)
            .build();

        let tex = device
            .create_texture2d(
                &desc,
                Some(&[D3D11_SUBRESOURCE_DATA {
                    pSysMem: data.as_ptr() as _,
                    SysMemPitch: format.calculate_pitch(width, height).0,
                    SysMemSlicePitch: 0,
                }]),
            )
            .with_context(|| format!("Failed to create 2D texture. Desc: {desc:#?}"))?;

        if let Some(name) = name {
            tex.set_debug_name(name);
        }

        let view = device.create_shader_resource_view(&tex, None)?;

        Ok(Texture {
            handle: TextureHandle::Texture2D(tex),
            view,
            format,
            width,
            height,
        })
    }

    pub fn load_3d_raw(
        device: &d3d11::Device,
        width: u32,
        height: u32,
        depth: u32,
        data: &[u8],
        format: impl Into<DxgiFormat>,
        name: Option<&str>,
    ) -> anyhow::Result<Texture> {
        let format = format.into();
        let tex = device.create_texture3d(
            &Texture3dDesc::builder()
                .width(width)
                .height(height)
                .depth(depth)
                .mip_levels(1)
                .format(format.into())
                .usage(d3d11::Usage::Default)
                .bind_flags(BindFlags::SHADER_RESOURCE)
                .build(),
            Some(&[D3D11_SUBRESOURCE_DATA {
                pSysMem: data.as_ptr() as _,
                SysMemPitch: format.calculate_pitch(width, height).0,
                SysMemSlicePitch: format.calculate_pitch(width, height).1,
            }]),
        )?;

        if let Some(name) = name {
            tex.set_debug_name(name);
        }

        let view = device.create_shader_resource_view(&tex, None)?;

        Ok(Texture {
            handle: TextureHandle::Texture3D(tex),
            view,
            format,
            width,
            height,
        })
    }

    pub fn load_2d_dds(device: &d3d11::Device, data: &[u8]) -> anyhow::Result<Texture> {
        let mut c = Cursor::new(data);
        let dds = ddsfile::Dds::read(&mut c)?;

        let format = DxgiFormat::from(
            convert_dds_texture_format(&dds).context("DDS file uses an invalid format")?,
        );

        let texture_data = &dds.data;
        let mut initial_data = Vec::with_capacity(dds.get_num_mipmap_levels() as usize);
        let mut offset = 0;
        for i in 0..dds.get_num_mipmap_levels() {
            let width = dds.get_width() >> i;
            let height = dds.get_height() >> i;
            let (pitch, slice_pitch) = format.calculate_pitch(width, height);

            initial_data.push(D3D11_SUBRESOURCE_DATA {
                pSysMem: unsafe { texture_data.as_ptr().add(offset) } as _,
                SysMemPitch: pitch,
                SysMemSlicePitch: 0,
            });
            offset += slice_pitch as usize;
        }

        let tex = device.create_texture2d(
            &Texture2dDesc::builder()
                .array_size(dds.get_num_array_layers())
                .mip_levels(dds.get_num_mipmap_levels())
                .width(dds.get_width())
                .height(dds.get_height())
                .format(format.into())
                .bind_flags(BindFlags::SHADER_RESOURCE)
                .build(),
            Some(&initial_data),
        )?;

        let view = device.create_shader_resource_view(&tex, None)?;

        Ok(Texture {
            view,
            handle: TextureHandle::Texture2D(tex),
            format,
            width: dds.get_width(),
            height: dds.get_height(),
        })
    }

    pub fn load_3d_dds(device: &d3d11::Device, data: &[u8]) -> anyhow::Result<Texture> {
        let mut c = Cursor::new(data);
        let dds = ddsfile::Dds::read(&mut c)?;

        let format = DxgiFormat::from(
            convert_dds_texture_format(&dds).context("DDS file uses an invalid format")?,
        );

        let texture_data = &dds.data;
        let mut initial_data = Vec::with_capacity(dds.get_num_mipmap_levels() as usize);
        let (pitch, slice_pitch) = format.calculate_pitch(dds.get_width(), dds.get_height());
        initial_data.push(D3D11_SUBRESOURCE_DATA {
            pSysMem: texture_data.as_ptr() as _,
            SysMemPitch: pitch,
            SysMemSlicePitch: slice_pitch,
        });
        // let mut offset = 0;
        // for i in 0..dds.get_num_mipmap_levels() {
        //     let width = dds.get_width() >> i;
        //     let height = dds.get_height() >> i;
        //     let depth = dds.get_depth() >> i;
        //     let (pitch, slice_pitch) = format_d3d.calculate_pitch(width as u32, height as u32);

        //     initial_data.push(D3D11_SUBRESOURCE_DATA {
        //         pSysMem: unsafe { texture_data.as_ptr().add(offset) } as _,
        //         SysMemPitch: pitch as u32,
        //         SysMemSlicePitch: slice_pitch as u32 * depth,
        //     });
        //     offset += slice_pitch as usize;
        // }

        let tex = device.create_texture3d(
            &Texture3dDesc::builder()
                .mip_levels(dds.get_num_mipmap_levels())
                .width(dds.get_width())
                .height(dds.get_height())
                .depth(dds.get_depth())
                .format(format.into())
                .bind_flags(BindFlags::SHADER_RESOURCE)
                .build(),
            Some(&initial_data),
        )?;

        let view = device.create_shader_resource_view(&tex, None)?;

        Ok(Texture {
            view,
            handle: TextureHandle::Texture3D(tex),
            format,
            width: dds.get_width(),
            height: dds.get_height(),
        })
    }

    // pub fn load_png(
    //     device: &d3d11::Device,
    //     png: &Png,
    //     name: Option<&str>,
    // ) -> anyhow::Result<Texture> {
    //     let converted_rgba = if png.color_type == png::ColorType::Rgba {
    //         None
    //     } else {
    //         Some(png.to_rgba()?)
    //     };

    //     Self::load_2d_raw(
    //         device,
    //         png.dimensions[0] as u32,
    //         png.dimensions[1] as u32,
    //         if let Some(p) = &converted_rgba {
    //             &p.data
    //         } else {
    //             &png.data
    //         },
    //         match png.bit_depth {
    //             png::BitDepth::Eight => dxgi::Format::R8G8B8A8_UNORM,
    //             png::BitDepth::Sixteen => dxgi::Format::R16G16B16A16_UNORM,
    //             u => todo!("Unsupported bit depth {u:?}"),
    //         },
    //         name,
    //     )
    // }

    pub fn bind(&self, cmd: &mut CommandList, slot: u32, stage: ShaderStage) {
        cmd.set_shader_resource(stage, slot, &self.view);
    }
}

pub fn load_sampler(gctx: &Gpu, hash: TagHash) -> anyhow::Result<d3d11::SamplerState> {
    let entry = package_manager()
        .get_entry(hash)
        .with_context(|| format!("Sampler entry for {hash} not found"))?;
    ensure!(
        entry.file_type == 34 && entry.file_subtype == 1,
        "Sampler header type mismatch"
    );
    let sampler_header_ref = entry.reference;
    let sampler_data = package_manager()
        .read_tag(sampler_header_ref)
        .context("Failed to read sampler data")?;

    anyhow::ensure!(
        sampler_data.len() >= std::mem::size_of::<SamplerDesc>(),
        "Sampler data size mismatch"
    );

    let sampler =
        unsafe { gctx.create_sampler_state(&*sampler_data.as_ptr().cast::<SamplerDesc>())? };

    Ok(sampler)
}

fn convert_dds_texture_format(dds: &Dds) -> Option<dxgi::Format> {
    if let Some(dxgi_format) = dds.get_dxgi_format() {
        dxgi::Format::try_from(dxgi_format as u32).ok()
    } else if let Some(d3d_format) = dds.get_d3d_format() {
        Some(match d3d_format {
            ddsfile::D3DFormat::A8B8G8R8 => dxgi::Format::R8g8b8a8Unorm,
            ddsfile::D3DFormat::G16R16 => dxgi::Format::R16g16Unorm,
            ddsfile::D3DFormat::A2B10G10R10 => dxgi::Format::R10g10b10a2Unorm,
            ddsfile::D3DFormat::A1R5G5B5 => dxgi::Format::B5g5r5a1Unorm,
            ddsfile::D3DFormat::R5G6B5 => dxgi::Format::B5g6r5Unorm,
            ddsfile::D3DFormat::A8 => dxgi::Format::A8Unorm,
            ddsfile::D3DFormat::A8R8G8B8 => dxgi::Format::B8g8r8a8Unorm,
            ddsfile::D3DFormat::X8R8G8B8 => dxgi::Format::B8g8r8x8Unorm,
            ddsfile::D3DFormat::A2R10G10B10 => dxgi::Format::R10g10b10a2Unorm,
            ddsfile::D3DFormat::A4R4G4B4 => dxgi::Format::B4g4r4a4Unorm,
            ddsfile::D3DFormat::A8L8 => dxgi::Format::R8g8Unorm,
            ddsfile::D3DFormat::L16 => dxgi::Format::R16Unorm,
            ddsfile::D3DFormat::L8 => dxgi::Format::R8Unorm,
            ddsfile::D3DFormat::DXT1 => dxgi::Format::Bc1Unorm,
            ddsfile::D3DFormat::DXT3 => dxgi::Format::Bc2Unorm,
            ddsfile::D3DFormat::DXT5 => dxgi::Format::Bc3Unorm,
            ddsfile::D3DFormat::R8G8_B8G8 => dxgi::Format::G8r8G8b8Unorm,
            ddsfile::D3DFormat::G8R8_G8B8 => dxgi::Format::R8g8B8g8Unorm,
            ddsfile::D3DFormat::A16B16G16R16 => dxgi::Format::R16g16b16a16Unorm,
            ddsfile::D3DFormat::Q16W16V16U16 => dxgi::Format::R16g16b16a16Snorm,
            ddsfile::D3DFormat::R16F => dxgi::Format::R16Float,
            ddsfile::D3DFormat::G16R16F => dxgi::Format::R16g16Float,
            ddsfile::D3DFormat::A16B16G16R16F => dxgi::Format::R16g16b16a16Float,
            ddsfile::D3DFormat::R32F => dxgi::Format::R32Float,
            ddsfile::D3DFormat::G32R32F => dxgi::Format::R32g32Float,
            ddsfile::D3DFormat::A32B32G32R32F => dxgi::Format::R32g32b32a32Float,
            ddsfile::D3DFormat::DXT2 => dxgi::Format::Bc2Unorm,
            ddsfile::D3DFormat::DXT4 => dxgi::Format::Bc3Unorm,
            ddsfile::D3DFormat::YUY2 => dxgi::Format::Yuy2,
            ddsfile::D3DFormat::X8B8G8R8
            | ddsfile::D3DFormat::R8G8B8
            | ddsfile::D3DFormat::X1R5G5B5
            | ddsfile::D3DFormat::X4R4G4B4
            | ddsfile::D3DFormat::A8R3G3B2
            | ddsfile::D3DFormat::A4L4
            | ddsfile::D3DFormat::UYVY
            | ddsfile::D3DFormat::CXV8U8 => return None,
        })
    } else {
        None
    }
}
