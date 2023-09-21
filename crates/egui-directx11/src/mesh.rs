use egui::{epaint::Vertex, Mesh, Pos2, Rect, Rgba, TextureId};
use std::mem::size_of;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11Device, D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER,
    D3D11_BUFFER_DESC, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
};

use crate::RenderError;

pub struct GpuMesh {
    pub indices: Vec<u32>,
    pub vertices: Vec<GpuVertex>,
    pub clip: Rect,
    pub texture_id: TextureId,
}

impl GpuMesh {
    pub fn from_mesh((w, h): (f32, f32), mesh: Mesh, scissors: Rect) -> Option<Self> {
        if mesh.indices.is_empty() || mesh.indices.len() % 3 != 0 {
            None
        } else {
            let vertices = mesh
                .vertices
                .into_iter()
                .map(|v| GpuVertex {
                    pos: Pos2::new(
                        (v.pos.x - w / 2.) / (w / 2.),
                        (v.pos.y - h / 2.) / -(h / 2.),
                    ),
                    uv: v.uv,
                    color: v.color.into(),
                })
                .collect();

            Some(Self {
                texture_id: mesh.texture_id,
                indices: mesh.indices,
                clip: scissors,
                vertices,
            })
        }
    }
}

#[repr(C)]
pub struct GpuVertex {
    pos: Pos2,
    uv: Pos2,
    color: Rgba,
}

impl From<Vertex> for GpuVertex {
    fn from(v: Vertex) -> Self {
        Self {
            pos: v.pos,
            uv: v.uv,
            color: v.color.into(),
        }
    }
}

pub fn create_vertex_buffer(
    device: &ID3D11Device,
    mesh: &GpuMesh,
) -> Result<ID3D11Buffer, RenderError> {
    let desc = D3D11_BUFFER_DESC {
        ByteWidth: (mesh.vertices.len() * size_of::<GpuVertex>()) as u32,
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_VERTEX_BUFFER,
        ..Default::default()
    };

    let init = D3D11_SUBRESOURCE_DATA {
        pSysMem: mesh.vertices.as_ptr() as _,
        ..Default::default()
    };

    unsafe {
        let mut output = None;
        device.CreateBuffer(&desc, Some(&init), Some(&mut output))?;
        output.ok_or(RenderError::General("Failed to create vertex buffer"))
    }
}

pub fn create_index_buffer(
    device: &ID3D11Device,
    mesh: &GpuMesh,
) -> Result<ID3D11Buffer, RenderError> {
    let desc = D3D11_BUFFER_DESC {
        ByteWidth: (mesh.indices.len() * size_of::<u32>()) as u32,
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_INDEX_BUFFER,
        ..Default::default()
    };

    let init = D3D11_SUBRESOURCE_DATA {
        pSysMem: mesh.indices.as_ptr() as _,
        ..Default::default()
    };

    unsafe {
        let mut output = None;
        device.CreateBuffer(&desc, Some(&init), Some(&mut output))?;
        output.ok_or(RenderError::General("Failed to create index buffer"))
    }
}
