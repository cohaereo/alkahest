use alkahest_data::tfx::common::AxisAlignedBBox;
use anyhow::Context;
use d3d11::{InputElementDesc, dxgi, fxc::ShaderTarget};
use glam::{Vec3, Vec4Swizzles, vec3};

use crate::{Gpu, gpu::command_list::CommandList, gpu_span, visibility::frustum::Frustum};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ImmediateVertex {
    pos: Vec3,
    color: u32,
}

pub struct ImmediateShapeRenderer {
    line_vertices: Vec<ImmediateVertex>,
    vbuffer: d3d11::Buffer,
    vbuffer_capacity: usize,
    vbuffer_len: usize,

    shader_vs: d3d11::VertexShader,
    shader_ps: d3d11::PixelShader,
    input_layout: d3d11::InputLayout,
}

const IMMEDIATE_SHADER: &str = include_str!("../../builtin/shaders/immediate.hlsl");

impl ImmediateShapeRenderer {
    const DEFAULT_CAPACITY: usize = 2048;

    pub fn new(gpu: &Gpu) -> anyhow::Result<Self> {
        let vs_data = d3d11::fxc::compile(
            IMMEDIATE_SHADER.as_bytes(),
            Some("immediate_vs"),
            &[],
            "mainVS",
            ShaderTarget::Vertex,
        )
        .context("Failed to compile vertex shader")?;

        let ps_data = d3d11::fxc::compile(
            IMMEDIATE_SHADER.as_bytes(),
            Some("immediate_ps"),
            &[],
            "mainPS",
            ShaderTarget::Pixel,
        )
        .context("Failed to compile pixel shader")?;
        let shader_vs = gpu
            .create_vertex_shader(&vs_data)
            .context("Failed to create vertex shader")?;
        let shader_ps = gpu
            .create_pixel_shader(&ps_data)
            .context("Failed to create pixel shader")?;

        let input_layout = gpu
            .create_input_layout(
                &[
                    InputElementDesc::builder()
                        .semantic_name("POSITION")
                        .semantic_index(0)
                        .format(dxgi::Format::R32g32b32Float)
                        .input_slot(0)
                        .input_slot_class(d3d11::InputClassification::PerVertexData)
                        .build(),
                    InputElementDesc::builder()
                        .semantic_name("COLOR")
                        .semantic_index(0)
                        .format(dxgi::Format::B8g8r8a8Unorm)
                        .input_slot(0)
                        .input_slot_class(d3d11::InputClassification::PerVertexData)
                        .build(),
                ],
                &vs_data,
            )
            .context("Failed to create input layout")?;

        Ok(Self {
            line_vertices: Vec::new(),
            vbuffer: Self::create_vb(gpu, Self::DEFAULT_CAPACITY),
            vbuffer_capacity: Self::DEFAULT_CAPACITY,
            vbuffer_len: 0,

            shader_vs,
            shader_ps,
            input_layout,
        })
    }

    fn create_vb(gpu: &Gpu, vertices: usize) -> d3d11::Buffer {
        gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width((vertices * std::mem::size_of::<ImmediateVertex>()) as u32)
                .usage(d3d11::Usage::Dynamic)
                .bind_flags(d3d11::BindFlags::VERTEX_BUFFER)
                .cpu_access_flags(d3d11::CpuAccessFlags::WRITE)
                .build(),
            None,
        )
        .unwrap()
    }

    // pub fn draw_line(&mut self, start: Vec3, end: Vec3, color: u32) {
    //     self.line_vertices.push(Vertex { pos: start, color });
    //     self.line_vertices.push(Vertex { pos: end, color });
    // }

    pub fn add_vertices(&mut self, vertices: &[ImmediateVertex]) {
        self.line_vertices.extend_from_slice(vertices);
    }

    #[profiling::function]
    pub fn prepare(&mut self, gpu: &crate::Gpu) {
        if self.line_vertices.len() > self.vbuffer_capacity {
            self.vbuffer = Self::create_vb(gpu, self.line_vertices.len());
            self.vbuffer_capacity = self.line_vertices.len();
        }

        unsafe {
            let ptr = gpu
                .context()
                .map(&self.vbuffer, 0, d3d11::MapType::WriteDiscard, false)
                .unwrap();

            std::ptr::copy_nonoverlapping(
                self.line_vertices.as_ptr(),
                ptr.data as *mut ImmediateVertex,
                self.line_vertices.len(),
            );
        }

        self.vbuffer_len = self.line_vertices.len();
        self.line_vertices.clear();
    }

    pub fn submit(&self, cmd: &mut CommandList) {
        gpu_span!();
        cmd.input_assembler_set_vertex_buffers(
            0,
            &[Some(&self.vbuffer)],
            Some(&[16]),
            Some(&[0u32]),
        )
        .unwrap();
        cmd.set_input_layout_custom(&self.input_layout);
        cmd.set_input_topology(alkahest_data::tfx::PrimitiveType::LineList);

        cmd.output_merger_set_blend_state(None, Some(&[1.0, 1.0, 1.0, 1.0]), 0xFFFF_FFFF);

        cmd.vertex_set_shader(&self.shader_vs);
        cmd.pixel_set_shader(&self.shader_ps);

        cmd.draw(self.vbuffer_len as u32, 0);
    }
}

// Shape methods
impl ImmediateShapeRenderer {
    #[inline]
    pub fn line(&mut self, start: impl Into<Vec3>, end: impl Into<Vec3>, color: u32) {
        self.add_vertices(&[
            ImmediateVertex {
                pos: start.into(),
                color,
            },
            ImmediateVertex {
                pos: end.into(),
                color,
            },
        ]);
    }

    pub fn cross(&mut self, center: Vec3, size: f32, color: u32) {
        let half_size = size / 2.0;
        let start = center - vec3(half_size, 0.0, 0.0);
        let end = center + vec3(half_size, 0.0, 0.0);
        self.line(start, end, color);

        let start = center - vec3(0.0, half_size, 0.0);
        let end = center + vec3(0.0, half_size, 0.0);
        self.line(start, end, color);

        let start = center - vec3(0.0, 0.0, half_size);
        let end = center + vec3(0.0, 0.0, half_size);
        self.line(start, end, color);
    }

    pub fn aabb_world(&mut self, bb: &AxisAlignedBBox, color: u32) {
        let a = bb.min.xyz();
        let b = bb.max.xyz();

        // +Z
        // ^  .1------b
        // |.' |    .'|
        // +------2'  | +X
        // |   |  |   | /
        // |  ,+--+---3
        // |.'    | .'
        // a------+'   -> -Y

        // Point axis (A)
        self.line(a, vec3(b.x, a.y, a.z), color); // X
        self.line(a, vec3(a.x, b.y, a.z), color); // Y
        self.line(a, vec3(a.x, a.y, b.z), color); // Z

        // Point axis (B)
        self.line(b, vec3(a.x, b.y, b.z), color); // X
        self.line(b, vec3(b.x, a.y, b.z), color); // Y
        self.line(b, vec3(b.x, b.y, a.z), color); // Z

        let c1 = vec3(b.x, a.y, b.z);
        let c2 = vec3(a.x, b.y, b.z);
        let c3 = vec3(b.x, b.y, a.z);

        // Infill corner 1
        self.line(c1, vec3(c1.x, c1.y, a.z), color);
        self.line(c1, vec3(a.x, c1.y, c1.z), color);

        // Infill corner 2
        self.line(c2, vec3(c2.x, a.y, c2.z), color);
        self.line(c2, vec3(c2.x, c2.y, a.z), color);

        // Infill corner 3
        self.line(c3, vec3(a.x, c3.y, c3.z), color);
        self.line(c3, vec3(c3.x, a.y, c3.z), color);
    }

    pub fn frustum(&mut self, frust: &Frustum, color: u32) {
        // Near
        self.line(frust.points[0], frust.points[1], color);
        self.line(frust.points[1], frust.points[2], color);
        self.line(frust.points[2], frust.points[3], color);
        self.line(frust.points[3], frust.points[0], color);

        // // Connecting lines
        self.line(frust.points[0], frust.points[4], color);
        self.line(frust.points[1], frust.points[5], color);
        self.line(frust.points[2], frust.points[6], color);
        self.line(frust.points[3], frust.points[7], color);

        // Far
        self.line(frust.points[4], frust.points[5], color);
        self.line(frust.points[5], frust.points[6], color);
        self.line(frust.points[6], frust.points[7], color);
        self.line(frust.points[7], frust.points[4], color);
    }
}
