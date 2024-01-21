use crate::dxgi::DxgiFormat;
use crate::render::DeviceContextSwapchain;
use anyhow::Context;
use std::mem::transmute;
use std::sync::Arc;
use windows::Win32::Graphics::Direct3D::{
    WKPDID_D3DDebugObjectName, D3D11_SRV_DIMENSION_TEXTURE2D, D3D11_SRV_DIMENSION_TEXTURE2DARRAY,
};
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;

use super::cbuffer::BufferMapGuard;

pub struct GBuffer {
    pub rt0: RenderTarget,
    pub rt1: RenderTarget,
    pub rt1_clone: RenderTarget,
    pub rt2: RenderTarget,
    pub rt3: RenderTarget,

    pub outline_depth: DepthState,
    pub pick_buffer: RenderTarget,
    pub pick_buffer_staging: CpuStagingBuffer,

    pub light_diffuse: RenderTarget,
    pub light_specular: RenderTarget,
    pub light_ibl_specular: RenderTarget,

    pub staging: RenderTarget,
    pub staging_clone: RenderTarget,
    pub depth: DepthState,
    dcs: Arc<DeviceContextSwapchain>,
}

impl GBuffer {
    pub fn create(size: (u32, u32), dcs: Arc<DeviceContextSwapchain>) -> anyhow::Result<Self> {
        Ok(Self {
            rt0: RenderTarget::create(size, DxgiFormat::B8G8R8A8_UNORM_SRGB, dcs.clone(), "RT0")
                .context("RT0")?,
            rt1: RenderTarget::create(size, DxgiFormat::R10G10B10A2_UNORM, dcs.clone(), "RT1")
                .context("RT1")?,
            rt1_clone: RenderTarget::create(
                size,
                DxgiFormat::R10G10B10A2_UNORM,
                dcs.clone(),
                "RT1_Clone",
            )
            .context("RT1_Clone")?,
            rt2: RenderTarget::create(size, DxgiFormat::B8G8R8A8_UNORM, dcs.clone(), "RT2")
                .context("RT2")?,
            rt3: RenderTarget::create(size, DxgiFormat::B8G8R8A8_UNORM, dcs.clone(), "RT3")
                .context("RT3")?,

            outline_depth: DepthState::create(size, &dcs.device).context("Outline Depth")?,
            pick_buffer: RenderTarget::create(
                size,
                DxgiFormat::R32_UINT,
                dcs.clone(),
                "Entity_Pickbuffer",
            )
            .context("Entity_Pickbuffer")?,
            pick_buffer_staging: CpuStagingBuffer::create(
                size,
                DxgiFormat::R32_UINT,
                dcs.clone(),
                "Entity_Pickbuffer_Staging",
            )
            .context("Entity_Pickbuffer_Staging")?,

            light_diffuse: RenderTarget::create(
                size,
                DxgiFormat::R16G16B16A16_FLOAT,
                dcs.clone(),
                "Light_Diffuse",
            )
            .context("Light_Diffuse")?,
            light_specular: RenderTarget::create(
                size,
                DxgiFormat::R16G16B16A16_FLOAT,
                dcs.clone(),
                "Light_Specular",
            )
            .context("Light_Specular")?,
            light_ibl_specular: RenderTarget::create(
                size,
                DxgiFormat::B8G8R8A8_UNORM_SRGB,
                dcs.clone(),
                "Specular_IBL",
            )
            .context("Specular_IBL")?,

            staging: RenderTarget::create(
                size,
                DxgiFormat::B8G8R8A8_UNORM_SRGB,
                dcs.clone(),
                "Staging",
            )
            .context("Staging")?,
            staging_clone: RenderTarget::create(
                size,
                DxgiFormat::B8G8R8A8_UNORM_SRGB,
                dcs.clone(),
                "Staging_Clone",
            )
            .context("Staging_Clone")?,
            depth: DepthState::create(size, &dcs.device).context("Depth")?,
            dcs,
        })
    }

    pub fn resize(&mut self, new_size: (u32, u32)) -> anyhow::Result<()> {
        if new_size.0 == 0 || new_size.1 == 0 {
            return Ok(());
        }

        self.rt0.resize(new_size).context("RT0")?;
        self.rt1.resize(new_size).context("RT1")?;
        self.rt1_clone.resize(new_size).context("RT1_Clone")?;
        self.rt2.resize(new_size).context("RT2")?;
        self.rt3.resize(new_size).context("RT3")?;

        self.outline_depth
            .resize(new_size, &self.dcs.device)
            .context("Outline Depth")?;

        self.pick_buffer
            .resize(new_size)
            .context("Entity_Pickbuffer")?;
        self.pick_buffer_staging
            .resize(new_size)
            .context("Entity_Pickbuffer_Staging")?;

        self.light_diffuse
            .resize(new_size)
            .context("Light_Diffuse")?;
        self.light_specular
            .resize(new_size)
            .context("Light_Specular")?;
        self.light_ibl_specular
            .resize(new_size)
            .context("Specular_IBL")?;

        self.staging.resize(new_size).context("Staging")?;
        self.staging_clone
            .resize(new_size)
            .context("Staging_Clone")?;
        self.depth
            .resize(new_size, &self.dcs.device)
            .context("Depth")?;

        Ok(())
    }
}

pub struct RenderTarget {
    pub texture: ID3D11Texture2D,
    pub render_target: ID3D11RenderTargetView,
    pub view: ID3D11ShaderResourceView,
    pub format: DxgiFormat,
    pub name: String,

    dcs: Arc<DeviceContextSwapchain>,
}

impl RenderTarget {
    pub fn create(
        size: (u32, u32),
        format: DxgiFormat,
        dcs: Arc<DeviceContextSwapchain>,
        name: &str,
    ) -> anyhow::Result<Self> {
        unsafe {
            let texture = dcs
                .device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: size.0,
                        Height: size.1,
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: DXGI_FORMAT(format as u32),
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE,
                        CPUAccessFlags: Default::default(),
                        MiscFlags: Default::default(),
                    },
                    None,
                )
                .context("Failed to create texture")?;

            let render_target = dcs
                .device
                .CreateRenderTargetView(&texture, None)
                .context("Failed to create RTV")?;
            let view = dcs
                .device
                .CreateShaderResourceView(
                    &texture,
                    Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                        Format: DXGI_FORMAT(format as u32),
                        ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                            Texture2D: D3D11_TEX2D_SRV {
                                MostDetailedMip: 0,
                                MipLevels: 1,
                            },
                        },
                    }),
                )
                .context("Failed to create SRV")?;

            let name = format!("{name}\0");
            texture
                .SetPrivateData(
                    &WKPDID_D3DDebugObjectName,
                    name.len() as u32,
                    Some(name.as_ptr() as _),
                )
                .ok();

            Ok(Self {
                texture,
                render_target,
                view,
                format,
                name: name.to_string(),
                dcs,
            })
        }
    }

    pub fn copy_to(&self, dest: &RenderTarget) {
        unsafe {
            self.dcs
                .context()
                .CopyResource(&dest.texture, &self.texture)
        }
    }

    pub fn copy_to_staging(&self, dest: &CpuStagingBuffer) {
        unsafe {
            self.dcs
                .context()
                .CopyResource(&dest.texture, &self.texture)
        }
    }

    pub fn resize(&mut self, new_size: (u32, u32)) -> anyhow::Result<()> {
        *self = Self::create(new_size, self.format, self.dcs.clone(), &self.name)?;
        Ok(())
    }
}

pub struct CpuStagingBuffer {
    pub texture: ID3D11Texture2D,
    pub format: DxgiFormat,
    pub name: String,
    dcs: Arc<DeviceContextSwapchain>,
}

impl CpuStagingBuffer {
    pub fn create(
        size: (u32, u32),
        format: DxgiFormat,
        dcs: Arc<DeviceContextSwapchain>,
        name: &str,
    ) -> anyhow::Result<Self> {
        unsafe {
            let texture = dcs
                .device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: size.0,
                        Height: size.1,
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: DXGI_FORMAT(format as u32),
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Usage: D3D11_USAGE_STAGING,
                        BindFlags: Default::default(),
                        CPUAccessFlags: D3D11_CPU_ACCESS_READ,
                        MiscFlags: Default::default(),
                    },
                    None,
                )
                .context("Failed to create staging buffer")?;

            let name = format!("{name}\0");
            texture
                .SetPrivateData(
                    &WKPDID_D3DDebugObjectName,
                    name.len() as u32,
                    Some(name.as_ptr() as _),
                )
                .ok();

            Ok(Self {
                texture,
                format,
                name: name.to_string(),
                dcs,
            })
        }
    }

    pub fn resize(&mut self, new_size: (u32, u32)) -> anyhow::Result<()> {
        *self = Self::create(new_size, self.format, self.dcs.clone(), &self.name)?;
        Ok(())
    }

    pub fn map(&self, mode: D3D11_MAP) -> anyhow::Result<BufferMapGuard<u8>> {
        let ptr = unsafe {
            self.dcs
                .context()
                .Map(&self.texture, 0, mode, 0)
                .context("Failed to map CpuStagingBuffer")?
        };

        Ok(BufferMapGuard {
            ptr: ptr.pData as _,
            row_pitch: ptr.RowPitch,
            depth_pitch: ptr.DepthPitch,
            resource: unsafe { transmute(&self.texture as *const ID3D11Texture2D as *const _) },
            dcs: self.dcs.clone(),
        })
    }
}

pub struct DepthState {
    pub texture: ID3D11Texture2D,
    // TODO(cohae): Should this be here?
    pub state: ID3D11DepthStencilState,
    pub state_readonly: ID3D11DepthStencilState,
    pub view: ID3D11DepthStencilView,
    pub texture_view: ID3D11ShaderResourceView,

    pub texture_copy: ID3D11Texture2D,
    pub texture_copy_view: ID3D11ShaderResourceView,
}

impl DepthState {
    pub fn create(size: (u32, u32), device: &ID3D11Device) -> anyhow::Result<Self> {
        let texture = unsafe {
            device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: size.0,
                        Height: size.1,
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: DXGI_FORMAT_R32_TYPELESS,
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: D3D11_BIND_DEPTH_STENCIL | D3D11_BIND_SHADER_RESOURCE,
                        CPUAccessFlags: Default::default(),
                        MiscFlags: Default::default(),
                    },
                    None,
                )
                .context("Failed to create depth texture")?
        };

        let state = unsafe {
            device
                .CreateDepthStencilState(&D3D11_DEPTH_STENCIL_DESC {
                    DepthEnable: true.into(),
                    DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ALL,
                    DepthFunc: D3D11_COMPARISON_GREATER_EQUAL,
                    StencilEnable: false.into(),
                    StencilReadMask: 0xff,
                    StencilWriteMask: 0xff,
                    FrontFace: D3D11_DEPTH_STENCILOP_DESC {
                        StencilFailOp: D3D11_STENCIL_OP_KEEP,
                        StencilDepthFailOp: D3D11_STENCIL_OP_INCR,
                        StencilPassOp: D3D11_STENCIL_OP_KEEP,
                        StencilFunc: D3D11_COMPARISON_ALWAYS,
                    },
                    BackFace: D3D11_DEPTH_STENCILOP_DESC {
                        StencilFailOp: D3D11_STENCIL_OP_KEEP,
                        StencilDepthFailOp: D3D11_STENCIL_OP_DECR,
                        StencilPassOp: D3D11_STENCIL_OP_KEEP,
                        StencilFunc: D3D11_COMPARISON_ALWAYS,
                    },
                })
                .context("Failed to create depth stencil state")?
        };

        let state_readonly = unsafe {
            device
                .CreateDepthStencilState(&D3D11_DEPTH_STENCIL_DESC {
                    DepthEnable: true.into(),
                    DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ZERO,
                    DepthFunc: D3D11_COMPARISON_GREATER_EQUAL,
                    StencilEnable: false.into(),
                    StencilReadMask: 0xff,
                    StencilWriteMask: 0xff,
                    FrontFace: D3D11_DEPTH_STENCILOP_DESC {
                        StencilFailOp: D3D11_STENCIL_OP_KEEP,
                        StencilDepthFailOp: D3D11_STENCIL_OP_INCR,
                        StencilPassOp: D3D11_STENCIL_OP_KEEP,
                        StencilFunc: D3D11_COMPARISON_ALWAYS,
                    },
                    BackFace: D3D11_DEPTH_STENCILOP_DESC {
                        StencilFailOp: D3D11_STENCIL_OP_KEEP,
                        StencilDepthFailOp: D3D11_STENCIL_OP_DECR,
                        StencilPassOp: D3D11_STENCIL_OP_KEEP,
                        StencilFunc: D3D11_COMPARISON_ALWAYS,
                    },
                })
                .context("Failed to create read-only depth stencil state")?
        };

        let view = unsafe {
            device
                .CreateDepthStencilView(
                    &texture,
                    Some(&D3D11_DEPTH_STENCIL_VIEW_DESC {
                        Format: DXGI_FORMAT_D32_FLOAT,
                        ViewDimension: D3D11_DSV_DIMENSION_TEXTURE2D,
                        Flags: 0,
                        Anonymous: D3D11_DEPTH_STENCIL_VIEW_DESC_0 {
                            Texture2D: { D3D11_TEX2D_DSV { MipSlice: 0 } },
                        },
                    }),
                )
                .context("Failed to create depth stencil view")?
        };

        let texture_view = unsafe {
            device.CreateShaderResourceView(
                &texture,
                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                    Format: DXGI_FORMAT_R32_FLOAT,
                    ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                        Texture2D: D3D11_TEX2D_SRV {
                            MostDetailedMip: 0,
                            MipLevels: 1,
                        },
                    },
                }),
            )?
        };

        let texture_copy = unsafe {
            device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: size.0,
                        Height: size.1,
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: DXGI_FORMAT_R32_TYPELESS,
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: D3D11_BIND_SHADER_RESOURCE,
                        CPUAccessFlags: Default::default(),
                        MiscFlags: Default::default(),
                    },
                    None,
                )
                .context("Failed to create depth texture")?
        };

        let texture_copy_view = unsafe {
            device.CreateShaderResourceView(
                &texture_copy,
                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                    Format: DXGI_FORMAT_R32_FLOAT,
                    ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                        Texture2D: D3D11_TEX2D_SRV {
                            MostDetailedMip: 0,
                            MipLevels: 1,
                        },
                    },
                }),
            )?
        };

        Ok(Self {
            texture,
            state,
            state_readonly,
            view,
            texture_view,
            texture_copy,
            texture_copy_view,
        })
    }

    /// Copies the depth texture to texture_copy
    pub fn copy_depth(&self, context: &ID3D11DeviceContext) {
        unsafe {
            context.CopyResource(&self.texture_copy, &self.texture);
        }
    }

    pub fn resize(&mut self, new_size: (u32, u32), device: &ID3D11Device) -> anyhow::Result<()> {
        *self = Self::create(new_size, device)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ShadowDepthMap {
    pub texture: ID3D11Texture2D,
    pub state: ID3D11DepthStencilState,
    pub views: Vec<ID3D11DepthStencilView>,
    pub texture_view: ID3D11ShaderResourceView,
    pub layers: usize,
}

impl ShadowDepthMap {
    pub fn create(size: (u32, u32), layers: usize, device: &ID3D11Device) -> anyhow::Result<Self> {
        let texture = unsafe {
            device
                .CreateTexture2D(
                    &D3D11_TEXTURE2D_DESC {
                        Width: size.0,
                        Height: size.1,
                        MipLevels: 1,
                        ArraySize: layers as u32,
                        Format: DXGI_FORMAT_R32_TYPELESS,
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: D3D11_BIND_DEPTH_STENCIL | D3D11_BIND_SHADER_RESOURCE,
                        CPUAccessFlags: Default::default(),
                        MiscFlags: Default::default(),
                    },
                    None,
                )
                .context("Failed to create depth texture")?
        };

        let state = unsafe {
            device
                .CreateDepthStencilState(&D3D11_DEPTH_STENCIL_DESC {
                    DepthEnable: true.into(),
                    DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ALL,
                    DepthFunc: D3D11_COMPARISON_LESS_EQUAL,
                    StencilEnable: false.into(),
                    StencilReadMask: 0xff,
                    StencilWriteMask: 0xff,
                    FrontFace: D3D11_DEPTH_STENCILOP_DESC {
                        StencilFailOp: D3D11_STENCIL_OP_KEEP,
                        StencilDepthFailOp: D3D11_STENCIL_OP_INCR,
                        StencilPassOp: D3D11_STENCIL_OP_KEEP,
                        StencilFunc: D3D11_COMPARISON_ALWAYS,
                    },
                    BackFace: D3D11_DEPTH_STENCILOP_DESC {
                        StencilFailOp: D3D11_STENCIL_OP_KEEP,
                        StencilDepthFailOp: D3D11_STENCIL_OP_DECR,
                        StencilPassOp: D3D11_STENCIL_OP_KEEP,
                        StencilFunc: D3D11_COMPARISON_ALWAYS,
                    },
                })
                .context("Failed to create depth stencil state")?
        };

        let mut views = vec![];

        for i in 0..layers {
            let view = unsafe {
                device
                    .CreateDepthStencilView(
                        &texture,
                        Some(&D3D11_DEPTH_STENCIL_VIEW_DESC {
                            Format: DXGI_FORMAT_D32_FLOAT,
                            ViewDimension: D3D11_DSV_DIMENSION_TEXTURE2DARRAY,
                            Flags: 0,
                            Anonymous: D3D11_DEPTH_STENCIL_VIEW_DESC_0 {
                                Texture2DArray: {
                                    D3D11_TEX2D_ARRAY_DSV {
                                        MipSlice: 0,
                                        ArraySize: 1,
                                        FirstArraySlice: i as u32,
                                    }
                                },
                            },
                        }),
                    )
                    .context("Failed to create depth stencil view")?
            };

            views.push(view);
        }

        let texture_view = unsafe {
            device.CreateShaderResourceView(
                &texture,
                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                    Format: DXGI_FORMAT_R32_FLOAT,
                    ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2DARRAY,
                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                        Texture2DArray: D3D11_TEX2D_ARRAY_SRV {
                            MostDetailedMip: 0,
                            MipLevels: 1,
                            FirstArraySlice: 0,
                            ArraySize: layers as u32,
                        },
                    },
                }),
            )?
        };

        Ok(Self {
            texture,
            state,
            views,
            texture_view,
            layers,
        })
    }

    pub fn resize(&mut self, new_size: (u32, u32), device: &ID3D11Device) -> anyhow::Result<()> {
        *self = Self::create(new_size, self.layers, device)?;
        Ok(())
    }
}
