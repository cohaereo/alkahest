use crate::dxgi::DxgiFormat;
use crate::render::DeviceContextSwapchain;
use anyhow::Context;
use std::sync::Arc;
use windows::Win32::Graphics::Direct3D::D3D11_SRV_DIMENSION_TEXTURE2D;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;

pub struct GBuffer {
    pub rt0: RenderTarget,
    pub rt1: RenderTarget,
    pub rt2: RenderTarget,
    pub staging: RenderTarget,
    pub depth: DepthState,
    dcs: Arc<DeviceContextSwapchain>,
}

impl GBuffer {
    pub fn create(size: (u32, u32), dcs: Arc<DeviceContextSwapchain>) -> anyhow::Result<Self> {
        Ok(Self {
            rt0: RenderTarget::create(size, &dcs.device, DxgiFormat::B8G8R8A8_UNORM_SRGB)
                .context("RT0")?,
            rt1: RenderTarget::create(size, &dcs.device, DxgiFormat::R10G10B10A2_UNORM)
                .context("RT1")?,
            rt2: RenderTarget::create(size, &dcs.device, DxgiFormat::B8G8R8A8_UNORM)
                .context("RT2")?,
            staging: RenderTarget::create(size, &dcs.device, DxgiFormat::B8G8R8A8_UNORM_SRGB)
                .context("Staging")?,
            depth: DepthState::create(size, &dcs.device).context("Depth")?,
            dcs,
        })
    }

    pub fn resize(&mut self, new_size: (u32, u32)) -> anyhow::Result<()> {
        if new_size.0 == 0 || new_size.1 == 0 {
            return Ok(());
        }

        self.rt0.resize(new_size, &self.dcs.device).context("RT0")?;
        self.rt1.resize(new_size, &self.dcs.device).context("RT1")?;
        self.rt2.resize(new_size, &self.dcs.device).context("RT2")?;
        self.staging
            .resize(new_size, &self.dcs.device)
            .context("Staging")?;
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
}

impl RenderTarget {
    pub fn create(
        size: (u32, u32),
        device: &ID3D11Device,
        format: DxgiFormat,
    ) -> anyhow::Result<Self> {
        unsafe {
            let texture = device
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

            let render_target = device
                .CreateRenderTargetView(&texture, None)
                .context("Failed to create RTV")?;
            let view = device
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

            Ok(Self {
                texture,
                render_target,
                view,
                format,
            })
        }
    }

    pub fn resize(&mut self, new_size: (u32, u32), device: &ID3D11Device) -> anyhow::Result<()> {
        *self = Self::create(new_size, device, self.format)?;
        Ok(())
    }
}

pub struct DepthState {
    pub texture: ID3D11Texture2D,
    // TODO(cohae): Should this be here?
    pub state: ID3D11DepthStencilState,
    pub state_readonly: ID3D11DepthStencilState,
    pub view: ID3D11DepthStencilView,
    pub texture_view: ID3D11ShaderResourceView,
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

        Ok(Self {
            texture,
            state,
            state_readonly,
            view,
            texture_view,
        })
    }

    pub fn resize(&mut self, new_size: (u32, u32), device: &ID3D11Device) -> anyhow::Result<()> {
        *self = Self::create(new_size, device)?;
        Ok(())
    }
}
