use std::sync::atomic::{AtomicBool, Ordering};

use alkahest_data::{dxgi::DxgiFormat, texture::STextureHeader, tfx::TfxShaderStage, WideHash};
use alkahest_pm::package_manager;
use anyhow::Context;
use tiger_parse::PackageManagerExt;
use tracing::{debug_span, error};
use windows::Win32::Graphics::{
    Direct3D::{
        D3D11_SRV_DIMENSION_TEXTURE2D, D3D11_SRV_DIMENSION_TEXTURE3D,
        D3D11_SRV_DIMENSION_TEXTURECUBE,
    },
    Direct3D11::{ID3D11ShaderResourceView, ID3D11Texture2D, ID3D11Texture3D, *},
    Dxgi::Common::*,
};

use crate::{
    gpu::GpuContext,
    util::{
        d3d::{calc_dx_subresource, D3dResource},
        image::Png,
    },
};

pub static LOW_RES: AtomicBool = AtomicBool::new(false);

pub enum TextureHandle {
    Texture2D(ID3D11Texture2D),
    TextureCube(ID3D11Texture2D),
    Texture3D(ID3D11Texture3D),
}

pub struct Texture {
    pub view: ID3D11ShaderResourceView,
    pub handle: TextureHandle,
    pub format: DxgiFormat,
}

impl Texture {
    pub fn load_data(
        hash: WideHash,
        load_full_mip: bool,
    ) -> anyhow::Result<(STextureHeader, Vec<u8>)> {
        let texture_header_ref = package_manager()
            .get_entry(hash)
            .context("Texture header entry not found")?
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

    pub fn load(device: &ID3D11Device, hash: WideHash) -> anyhow::Result<Texture> {
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
                let mut tex = None;
                device
                    .CreateTexture3D(
                        &D3D11_TEXTURE3D_DESC {
                            Width: texture.width as _,
                            Height: texture.height as _,
                            Depth: texture.depth as _,
                            MipLevels: 1,
                            Format: dxgi_to_win(texture.format),
                            Usage: D3D11_USAGE_DEFAULT,
                            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                            CPUAccessFlags: Default::default(),
                            MiscFlags: Default::default(),
                        },
                        Some([initial_data].as_ptr()),
                        Some(&mut tex),
                    )
                    .context("Failed to create 3D texture")?;

                let tex = tex.unwrap();

                tex.set_debug_name(&format!("Texture3D {hash}"));

                let mut view = None;
                device.CreateShaderResourceView(
                    &tex,
                    Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                        Format: dxgi_to_win(texture.format),
                        ViewDimension: D3D11_SRV_DIMENSION_TEXTURE3D,
                        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                            Texture3D: D3D11_TEX3D_SRV {
                                MostDetailedMip: 0,
                                MipLevels: 1,
                            },
                        },
                    }),
                    Some(&mut view),
                )?;

                let view = view.unwrap();

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
                        let (pitch, slice_pitch) = texture
                            .format
                            .calculate_pitch(width as usize, height as usize);

                        initial_data[calc_dx_subresource(i, e, mip_count)] =
                            D3D11_SUBRESOURCE_DATA {
                                pSysMem: texture_data.as_ptr().add(offset) as _,
                                SysMemPitch: pitch as u32,
                                SysMemSlicePitch: 0,
                            };
                        offset += slice_pitch;
                    }
                }

                let _span_load = debug_span!("Load texturecube").entered();
                let mut tex = None;
                device
                    .CreateTexture2D(
                        &D3D11_TEXTURE2D_DESC {
                            Width: texture.width as _,
                            Height: texture.height as _,
                            MipLevels: mip_count as _,
                            ArraySize: texture.array_size as _,
                            Format: dxgi_to_win(texture.format),
                            SampleDesc: DXGI_SAMPLE_DESC {
                                Count: 1,
                                Quality: 0,
                            },
                            Usage: D3D11_USAGE_DEFAULT,
                            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                            CPUAccessFlags: Default::default(),
                            MiscFlags: D3D11_RESOURCE_MISC_TEXTURECUBE.0 as u32,
                        },
                        Some(initial_data.as_ptr()),
                        Some(&mut tex),
                    )
                    .context("Failed to create texture cube")?;

                let tex = tex.unwrap();

                tex.set_debug_name(&format!("TextureCube {hash}"));

                let mut view = None;
                device
                    .CreateShaderResourceView(
                        &tex,
                        Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                            Format: dxgi_to_win(texture.format),
                            ViewDimension: D3D11_SRV_DIMENSION_TEXTURECUBE,
                            Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                                TextureCube: D3D11_TEXCUBE_SRV {
                                    MostDetailedMip: 0,
                                    MipLevels: mip_count as _,
                                },
                            },
                        }),
                        Some(&mut view),
                    )
                    .context("Failed to create texture cube SRV")?;

                let view = view.unwrap();

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
                    let (pitch, slice_pitch) = texture
                        .format
                        .calculate_pitch(width as usize, height as usize);

                    if pitch == 0 {
                        mipcount_fixed = i;
                        break;
                    }

                    initial_data.push(D3D11_SUBRESOURCE_DATA {
                        pSysMem: texture_data.as_ptr().add(offset) as _,
                        SysMemPitch: pitch as u32,
                        SysMemSlicePitch: 0,
                    });
                    offset += slice_pitch;
                }

                let mut verylowres_mip = 0;
                if LOW_RES.load(Ordering::Relaxed) {
                    // Remove everything but mips under 4x4
                    let mut new_data = vec![];
                    for i in 0..mipcount_fixed {
                        let width: u16 = texture.width >> i;
                        let height = texture.height >> i;
                        if width <= 4 || height <= 4 {
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
                let mut tex = None;
                device
                    .CreateTexture2D(
                        &D3D11_TEXTURE2D_DESC {
                            Width: (texture.width >> verylowres_mip) as _,
                            Height: (texture.height >> verylowres_mip) as _,
                            MipLevels: initial_data.len() as u32,
                            ArraySize: 1 as _,
                            Format: dxgi_to_win(texture.format),
                            SampleDesc: DXGI_SAMPLE_DESC {
                                Count: 1,
                                Quality: 0,
                            },
                            Usage: D3D11_USAGE_DEFAULT,
                            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                            CPUAccessFlags: Default::default(),
                            MiscFlags: Default::default(),
                        },
                        Some(initial_data.as_ptr()),
                        Some(&mut tex),
                    )
                    .context("Failed to create 2D texture")?;

                let tex = tex.unwrap();

                tex.set_debug_name(&format!("Texture2D {hash}"));

                let mut view = None;
                device.CreateShaderResourceView(
                    &tex,
                    Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                        Format: dxgi_to_win(texture.format),
                        ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                            Texture2D: D3D11_TEX2D_SRV {
                                MostDetailedMip: 0,
                                MipLevels: initial_data.len() as _,
                            },
                        },
                    }),
                    Some(&mut view),
                )?;

                let view = view.unwrap();

                (TextureHandle::Texture2D(tex), view)
            }
        };

        Ok(Texture {
            handle: tex,
            view,
            format: texture.format,
        })
    }

    pub fn load_2d_raw(
        device: &ID3D11Device,
        width: u32,
        height: u32,
        data: &[u8],
        format: DxgiFormat,
        name: Option<&str>,
    ) -> anyhow::Result<Texture> {
        unsafe {
            let mut tex = None;
            device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: width,
                        Height: height,
                        MipLevels: 1,
                        ArraySize: 1 as _,
                        Format: dxgi_to_win(format),
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                        CPUAccessFlags: Default::default(),
                        MiscFlags: Default::default(),
                    },
                    Some(&D3D11_SUBRESOURCE_DATA {
                        pSysMem: data.as_ptr() as _,
                        SysMemPitch: format.calculate_pitch(width as usize, height as usize).0 as _,
                        SysMemSlicePitch: 0,
                    }),
                    Some(&mut tex),
                )
                .context("Failed to create 2D texture")?;

            let tex = tex.unwrap();

            if let Some(name) = name {
                tex.set_debug_name(name);
            }

            let mut view = None;
            device.CreateShaderResourceView(
                &tex,
                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                    Format: dxgi_to_win(format),
                    ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                        Texture2D: D3D11_TEX2D_SRV {
                            MostDetailedMip: 0,
                            MipLevels: 1,
                        },
                    },
                }),
                Some(&mut view),
            )?;

            let view = view.unwrap();

            Ok(Texture {
                handle: TextureHandle::Texture2D(tex),
                view,
                format,
            })
        }
    }

    pub fn load_3d_raw(
        device: &ID3D11Device,
        width: u32,
        height: u32,
        depth: u32,
        data: &[u8],
        format: DxgiFormat,
        name: Option<&str>,
    ) -> anyhow::Result<Texture> {
        unsafe {
            let mut tex = None;
            device
                .CreateTexture3D(
                    &D3D11_TEXTURE3D_DESC {
                        Width: width,
                        Height: height,
                        Depth: depth,
                        MipLevels: 1,
                        Format: dxgi_to_win(format),
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                        CPUAccessFlags: Default::default(),
                        MiscFlags: Default::default(),
                    },
                    Some(&D3D11_SUBRESOURCE_DATA {
                        pSysMem: data.as_ptr() as _,
                        SysMemPitch: format.calculate_pitch(width as usize, height as usize).0 as _,
                        SysMemSlicePitch: format.calculate_pitch(width as usize, height as usize).1
                            as _,
                    }),
                    Some(&mut tex),
                )
                .context("Failed to create 3D texture")?;

            let tex = tex.unwrap();

            if let Some(name) = name {
                tex.set_debug_name(name);
            }

            let mut view = None;
            device.CreateShaderResourceView(
                &tex,
                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                    Format: dxgi_to_win(format),
                    ViewDimension: D3D11_SRV_DIMENSION_TEXTURE3D,
                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                        Texture3D: D3D11_TEX3D_SRV {
                            MostDetailedMip: 0,
                            MipLevels: 1,
                        },
                    },
                }),
                Some(&mut view),
            )?;

            let view = view.unwrap();

            Ok(Texture {
                handle: TextureHandle::Texture3D(tex),
                view,
                format,
            })
        }
    }

    pub fn load_png(
        device: &ID3D11Device,
        png: &Png,
        name: Option<&str>,
    ) -> anyhow::Result<Texture> {
        let converted_rgba = if png.color_type == png::ColorType::Rgba {
            None
        } else {
            Some(png.to_rgba()?)
        };

        Self::load_2d_raw(
            device,
            png.dimensions[0] as u32,
            png.dimensions[1] as u32,
            if let Some(p) = &converted_rgba {
                &p.data
            } else {
                &png.data
            },
            match png.bit_depth {
                png::BitDepth::Eight => DxgiFormat::R8G8B8A8_UNORM,
                png::BitDepth::Sixteen => DxgiFormat::R16G16B16A16_UNORM,
                u => todo!("Unsupported bit depth {u:?}"),
            },
            name,
        )
    }

    pub fn bind(&self, gctx: &GpuContext, slot: u32, stage: TfxShaderStage) {
        gctx.bind_srv(Some(self.view.clone()), slot, stage);
    }
}

fn dxgi_to_win(v: DxgiFormat) -> DXGI_FORMAT {
    unsafe { std::mem::transmute(v) }
}
