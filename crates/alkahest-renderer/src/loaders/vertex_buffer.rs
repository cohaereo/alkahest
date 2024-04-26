use alkahest_data::buffers::VertexBufferHeader;
use alkahest_pm::package_manager;
use anyhow::{Context};
use destiny_pkg::TagHash;
use tiger_parse::PackageManagerExt;
use windows::Win32::Graphics::{
    Direct3D::D3D11_SRV_DIMENSION_BUFFER,
    Direct3D11::{
        ID3D11Buffer, ID3D11ShaderResourceView, D3D11_BIND_SHADER_RESOURCE,
        D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC, D3D11_BUFFER_SRV, D3D11_BUFFER_SRV_0,
        D3D11_BUFFER_SRV_1, D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0,
        D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
    },
    Dxgi::Common::DXGI_FORMAT_R8G8B8A8_UNORM,
};

use crate::gpu::SharedGpuContext;

pub struct VertexBuffer {
    pub buffer: ID3D11Buffer,
    pub size: u32,
    pub stride: u32,
    /// Optional SRV for the buffer. Created for buffers with stride 4
    pub srv: Option<ID3D11ShaderResourceView>,
}

pub(crate) fn load_vertex_buffer(
    gctx: &SharedGpuContext,
    hash: TagHash,
) -> anyhow::Result<VertexBuffer> {
    let entry = package_manager()
        .get_entry(hash)
        .context("Entry not found")?;

    let header: VertexBufferHeader = package_manager()
        .read_tag_struct(hash)
        .context("Failed to read header data")?;
    let data = package_manager()
        .read_tag(entry.reference)
        .context("Failed to read buffer data")?;

    let bind_flags = if header.stride == 4 {
        D3D11_BIND_VERTEX_BUFFER | D3D11_BIND_SHADER_RESOURCE
    } else {
        D3D11_BIND_VERTEX_BUFFER
    };
    let mut buffer = None;
    unsafe {
        gctx.device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                ByteWidth: header.data_size,
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: bind_flags.0 as u32,
                CPUAccessFlags: 0,
                MiscFlags: 0,
                StructureByteStride: 0,
            },
            Some(&D3D11_SUBRESOURCE_DATA {
                pSysMem: data.as_ptr() as _,
                ..Default::default()
            }),
            Some(&mut buffer),
        )?;
    }
    let buffer = buffer.unwrap();

    let mut srv = None;
    if header.stride == 4 {
        unsafe {
            gctx.device.CreateShaderResourceView(
                &buffer,
                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    ViewDimension: D3D11_SRV_DIMENSION_BUFFER,
                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                        Buffer: D3D11_BUFFER_SRV {
                            Anonymous1: D3D11_BUFFER_SRV_0 { ElementOffset: 0 },
                            Anonymous2: D3D11_BUFFER_SRV_1 {
                                NumElements: data.len() as u32 / header.stride as u32,
                            },
                        },
                    },
                }),
                Some(&mut srv),
            )?;
        }
    }

    Ok(VertexBuffer {
        buffer,
        size: header.data_size,
        stride: header.stride as u32,
        srv,
    })
}
