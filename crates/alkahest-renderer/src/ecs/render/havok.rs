use std::io::Cursor;

use alkahest_data::{
    geometry::EPrimitiveType,
    tfx::{TfxFeatureRenderer, TfxRenderStage, TfxShaderStage},
};
use alkahest_pm::package_manager;
use anyhow::Context;
use destiny_havok::shape_collection;
use destiny_pkg::TagHash;
use glam::{Vec3, Vec4Swizzles};
use itertools::Itertools;

use crate::{
    ecs::{
        common::Hidden, render::static_geometry::StaticInstances, tags::NodeFilter,
        transform::Transform, Scene,
    },
    gpu::{buffer::ConstantBuffer, GpuContext, SharedGpuContext},
    gpu_event, include_dxbc,
    loaders::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer},
    renderer::{shader::ShaderProgram, Renderer},
    tfx::technique::ShaderModule,
    Color, ColorExt,
};

#[repr(C)]
struct HavokShapeScope {
    local_to_world: glam::Mat4,
    color: glam::Vec4,
}

pub struct HavokShapeRenderer {
    shader: ShaderProgram,

    vb: VertexBuffer,
    ib_sides: IndexBuffer,
    outline_index_count: u32,
    index_count: u32,

    cb_debug_shape: ConstantBuffer<HavokShapeScope>,
}

impl HavokShapeRenderer {
    pub fn new(gpu: SharedGpuContext, shape: &shape_collection::Shape) -> anyhow::Result<Self> {
        let (vertices, indices) = calculate_mesh_normals_flat(&shape.vertices, &shape.indices);
        let indices_outline = remove_diagonals_linegulate(&vertices, &indices);

        let ib_sides = IndexBuffer::load_u16(&gpu, &indices)?;

        let vertices_flattened = vertices
            .iter()
            // .flat_map(|(v, n)| vec![v.x, v.y, v.z, n.x, n.y, n.z])
            .flat_map(|(v, _n)| vec![v.x, v.y, v.z])
            .collect_vec();
        let vb =
            VertexBuffer::load_data(&gpu.device, bytemuck::cast_slice(&vertices_flattened), 12)?;

        Ok(Self {
            shader: ShaderProgram::load(
                &gpu,
                include_dxbc!(vs "debug/custom.hlsl"),
                None,
                include_dxbc!(ps "debug/custom.hlsl"),
            )
            .context("Failed to load shader")?,
            vb,
            ib_sides,
            outline_index_count: indices_outline.len() as _,
            index_count: indices.len() as _,
            cb_debug_shape: ConstantBuffer::create(gpu.clone(), None)?,
        })
    }

    pub fn draw(&self, gpu: &GpuContext, transform: &Transform, color: Color) {
        gpu_event!(gpu, "havok_shape");
        self.vb.bind_single(gpu, 0);
        self.ib_sides.bind(gpu);

        self.cb_debug_shape
            .write(&HavokShapeScope {
                local_to_world: transform.local_to_world(),
                color: color.to_vec4().xyz().extend(0.10),
            })
            .unwrap();

        self.cb_debug_shape.bind(0, TfxShaderStage::Vertex);
        self.cb_debug_shape.bind(0, TfxShaderStage::Pixel);
        self.shader.bind(gpu);

        gpu.set_input_layout(0);
        gpu.set_input_topology(EPrimitiveType::Triangles);
        gpu.set_blend_state(12);
        unsafe {
            gpu.context().DrawIndexed(self.index_count, 0, 0);
        }

        gpu.set_input_topology(EPrimitiveType::LineList);
        gpu.set_blend_state(1);
        unsafe {
            gpu.context().Draw(self.outline_index_count, 0);
        }
    }
}

pub fn remove_diagonals_linegulate(vertices: &[(Vec3, Vec3)], indices: &[u16]) -> Vec<u16> {
    let mut indices_outline = vec![];
    for i in indices.chunks_exact(3) {
        let i0 = i[0];
        let i1 = i[1];
        let i2 = i[2];

        let v0 = vertices[i0 as usize].0;
        let v1 = vertices[i1 as usize].0;
        let v2 = vertices[i2 as usize].0;

        //         0
        //         |\ edge_a
        //  edge c | \
        //         2--1
        //           edge_b
        let edge_a = (v0 - v1).length();
        let edge_b = (v1 - v2).length();
        let edge_c = (v2 - v0).length();

        if edge_a > edge_b && edge_a > edge_c {
            indices_outline.extend_from_slice(&[i0, i2, i2, i1]);
        } else if edge_b > edge_a && edge_b > edge_c {
            indices_outline.extend_from_slice(&[i0, i1, i0, i2]);
        } else if edge_c > edge_a && edge_c > edge_b {
            indices_outline.extend_from_slice(&[i0, i1, i1, i2]);
        } else {
            indices_outline.extend_from_slice(&[i0, i1, i1, i2, i2, i0])
        }
    }

    indices_outline
}

pub fn calculate_mesh_normals_flat(
    vertices: &[Vec3],
    indices: &[u16],
) -> (Vec<(Vec3, Vec3)>, Vec<u16>) {
    let mut new_vertices = vec![];
    let mut new_indices = vec![];

    for i in indices.chunks_exact(3) {
        let i0 = i[0];
        let i1 = i[1];
        let i2 = i[2];

        let v0 = vertices[i0 as usize];
        let v1 = vertices[i1 as usize];
        let v2 = vertices[i2 as usize];

        let normal = (v1 - v0).cross(v2 - v0).normalize();

        let index_start = new_vertices.len() as u16;

        new_vertices.push((v0, normal));
        new_vertices.push((v1, normal));
        new_vertices.push((v2, normal));

        new_indices.extend_from_slice(&[index_start, index_start + 1, index_start + 2]);
    }

    (new_vertices, new_indices)
}

pub fn draw_debugshapes_system(renderer: &Renderer, scene: &Scene, render_stage: TfxRenderStage) {
    if render_stage != TfxRenderStage::Transparents
        || !renderer.should_render(Some(render_stage), None)
    {
        return;
    }

    for (_e, (transform, shape, filter)) in scene
        .query::<(&Transform, &HavokShapeRenderer, Option<&NodeFilter>)>()
        .without::<&Hidden>()
        .iter()
    {
        let color = if let Some(filter) = filter {
            if !renderer.lastfilters.contains(&filter) {
                continue;
            }

            filter.color()
        } else {
            Color::WHITE
        };

        shape.draw(&renderer.gpu, transform, color);
    }
}
