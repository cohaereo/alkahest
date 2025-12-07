use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use egui::{epaint::Vertex, Mesh, Pos2, Rect, Rgba, TextureId};

use crate::RenderError;

pub struct GpuMesh {
    pub indices: Vec<u32>,
    pub vertices: Vec<GpuVertex>,
    pub clip: Rect,
    pub texture_id: TextureId,
}

impl GpuMesh {
    pub fn from_mesh(
        (w, h): (f32, f32),
        mesh: Mesh,
        scissors: Rect,
        pixels_per_point: f32,
    ) -> Option<Self> {
        if mesh.indices.is_empty() || !mesh.indices.len().is_multiple_of(3) {
            None
        } else {
            let vertices = mesh
                .vertices
                .into_iter()
                .map(|v| GpuVertex {
                    pos: Pos2::new(
                        ((v.pos.x * pixels_per_point) - w / 2.) / (w / 2.),
                        ((v.pos.y * pixels_per_point) - h / 2.) / -(h / 2.),
                    ),
                    uv: v.uv,
                    color: v.color.into(),
                })
                .collect();

            // Transform clip rect to physical pixels:
            let clip_min_x = (pixels_per_point * scissors.min.x).round();
            let clip_min_y = (pixels_per_point * scissors.min.y).round();
            let clip_max_x = (pixels_per_point * scissors.max.x).round();
            let clip_max_y = (pixels_per_point * scissors.max.y).round();

            Some(Self {
                texture_id: mesh.texture_id,
                indices: mesh.indices,
                clip: Rect {
                    min: Pos2::new(clip_min_x, clip_min_y),
                    max: Pos2::new(clip_max_x, clip_max_y),
                },
                vertices,
            })
        }
    }
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy)]
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
    device: &d3d11::Device,
    mesh: &GpuMesh,
) -> Result<d3d11::Buffer, RenderError> {
    let desc = d3d11::BufferDesc::builder()
        .byte_width((mesh.vertices.len() * size_of::<GpuVertex>()) as u32)
        .usage(d3d11::Usage::Default)
        .bind_flags(d3d11::BindFlags::VERTEX_BUFFER)
        .build();

    device
        .create_buffer(&desc, Some(bytemuck::cast_slice(&mesh.vertices)))
        .map_err(|_| RenderError::General("Failed to create vertex buffer"))
}

pub fn create_index_buffer(
    device: &d3d11::Device,
    mesh: &GpuMesh,
) -> Result<d3d11::Buffer, RenderError> {
    let desc = d3d11::BufferDesc::builder()
        .byte_width((mesh.indices.len() * size_of::<u32>()) as u32)
        .usage(d3d11::Usage::Default)
        .bind_flags(d3d11::BindFlags::INDEX_BUFFER)
        .build();

    device
        .create_buffer(&desc, Some(bytemuck::cast_slice(&mesh.indices)))
        .map_err(|_| RenderError::General("Failed to create index buffer"))
}
