use alkahest_data::{geometry::EPrimitiveType, tfx::TfxShaderStage};
use crossbeam::epoch::Shared;
use genmesh::{
    generators::{IndexedPolygon, SharedVertex},
    Triangulate,
};
use glam::{Mat4, Vec3, Vec4};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;

use crate::{
    ecs::transform::Transform,
    gpu::{buffer::ConstantBuffer, GpuContext, SharedGpuContext},
    gpu_event, include_dxbc,
    loaders::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer},
    renderer::shader::ShaderProgram,
    util::{color::Color, mat4_scale_translation},
};

#[repr(C)]
struct ScopeAlkDebugShape {
    local_to_world: Mat4,
    color: Vec4,
}

#[repr(C)]
struct ScopeAlkDebugLine {
    line_start: Vec4,
    line_end: Vec4,

    color_start: Vec4,
    color_end: Vec4,

    width: f32,
    dot_scale: f32,
    line_ratio: f32,
    scroll_speed: f32,
}

pub struct ImmediateRenderer {
    gpu: SharedGpuContext,

    vb_cube: VertexBuffer,
    ib_cube: IndexBuffer,
    ib_cube_outline: IndexBuffer,

    vb_sphere: VertexBuffer,
    ib_sphere: IndexBuffer,

    shader_simple: ShaderProgram,
    shader_line: ShaderProgram,

    cb_debug_shape: ConstantBuffer<ScopeAlkDebugShape>,
    cb_debug_line: ConstantBuffer<ScopeAlkDebugLine>,
}

impl ImmediateRenderer {
    pub fn new(gpu: SharedGpuContext) -> anyhow::Result<Self> {
        let mesh_sphere = genmesh::generators::SphereUv::new(32, 32);
        let vertices: Vec<[f32; 4]> = mesh_sphere
            .shared_vertex_iter()
            .map(|v| {
                let v = <[f32; 3]>::from(v.pos);
                [v[0], v[1], v[2], 1.0]
            })
            .collect();

        let mut indices = vec![];
        for i in mesh_sphere.indexed_polygon_iter().triangulate() {
            indices.extend_from_slice(&[i.x as u16, i.y as u16, i.z as u16]);
        }

        let ib_sphere = IndexBuffer::load_u16(&gpu, &indices)?;

        let vb_sphere = VertexBuffer::load_data(
            &gpu.device,
            bytemuck::cast_slice(&vertices),
            std::mem::size_of::<[f32; 4]>() as u32,
        )?;

        let mesh = genmesh::generators::Cube::new();
        let vertices: Vec<[f32; 4]> = mesh
            .shared_vertex_iter()
            .map(|v| {
                let v = <[f32; 3]>::from(v.pos);
                [v[0], v[1], v[2], 1.0]
            })
            .collect();
        let mut indices = vec![];
        let mut indices_outline = vec![];
        for i in mesh.indexed_polygon_iter().triangulate() {
            indices.extend_from_slice(&[i.x as u16, i.y as u16, i.z as u16]);
        }

        for i in mesh.indexed_polygon_iter() {
            indices_outline.extend_from_slice(&[
                i.x as u16, i.y as u16, i.y as u16, i.z as u16, i.z as u16, i.w as u16, i.w as u16,
                i.x as u16,
            ]);
        }

        let ib_cube = IndexBuffer::load_u16(&gpu, &indices)?;
        let ib_cube_outline = IndexBuffer::load_u16(&gpu, &indices_outline)?;

        let vb_cube = VertexBuffer::load_data(
            &gpu.device,
            bytemuck::cast_slice(&vertices),
            std::mem::size_of::<[f32; 4]>() as u32,
        )?;

        Ok(Self {
            vb_cube,
            ib_cube,
            ib_cube_outline,
            vb_sphere,
            ib_sphere,
            shader_simple: ShaderProgram::load(
                &gpu,
                include_dxbc!(vs "debug/simple.hlsl"),
                None,
                include_dxbc!(ps "debug/simple.hlsl"),
            )?,
            shader_line: ShaderProgram::load(
                &gpu,
                include_dxbc!(vs "debug/line.hlsl"),
                Some(include_dxbc!(gs "debug/line.hlsl")),
                include_dxbc!(ps "debug/line.hlsl"),
            )?,
            cb_debug_shape: ConstantBuffer::create(gpu.clone(), None)?,
            cb_debug_line: ConstantBuffer::create(gpu.clone(), None)?,
            gpu,
        })
    }

    pub fn line<C: Into<Color> + Copy>(&self, start: Vec3, end: Vec3, color: C, width: f32) {
        self.line_2color(start, end, color, color, width);
    }

    pub fn line_2color<C: Into<Color>>(
        &self,
        start: Vec3,
        end: Vec3,
        start_color: C,
        end_color: C,
        width: f32,
    ) {
        gpu_event!(self.gpu, "imm_line");
        let start_color = start_color.into();
        let end_color = end_color.into();
        self.shader_line.bind(&self.gpu);

        self.cb_debug_line
            .write(&ScopeAlkDebugLine {
                line_start: start.extend(1.0),
                line_end: end.extend(1.0),
                color_start: start_color.0,
                color_end: end_color.0,
                width,
                dot_scale: 1.0,
                line_ratio: 0.5,
                scroll_speed: 1.0,
            })
            .unwrap();

        self.cb_debug_line.bind(0, TfxShaderStage::Vertex);
        self.cb_debug_line.bind(0, TfxShaderStage::Geometry);
        self.cb_debug_line.bind(0, TfxShaderStage::Pixel);

        self.gpu.set_input_layout(0);
        self.gpu.set_input_topology(EPrimitiveType::LineList);
        if !start_color.is_opaque() || !end_color.is_opaque() {
            self.gpu.set_blend_state(8);
        } else {
            self.gpu.set_blend_state(0);
        }

        unsafe {
            self.gpu.context().Draw(2, 0);
            self.gpu.context().GSSetShader(None, None);
        }
    }

    pub fn sphere<C: Into<Color>>(&self, center: Vec3, radius: f32, color: C) {
        gpu_event!(self.gpu, "imm_sphere");
        let color = color.into();
        self.shader_simple.bind(&self.gpu);

        self.vb_sphere.bind_single(&self.gpu);
        self.ib_sphere.bind(&self.gpu);

        self.cb_debug_shape
            .write(&ScopeAlkDebugShape {
                local_to_world: mat4_scale_translation(Vec3::splat(radius), center),
                color: color.0,
            })
            .unwrap();

        self.cb_debug_shape.bind(0, TfxShaderStage::Vertex);
        self.cb_debug_shape.bind(0, TfxShaderStage::Pixel);

        self.gpu.set_input_layout(0);
        self.gpu.set_input_topology(EPrimitiveType::Triangles);
        if color.is_opaque() {
            self.gpu.set_blend_state(0);
        } else {
            self.gpu.set_blend_state(8);
        }

        unsafe {
            self.gpu
                .context()
                .DrawIndexed(self.ib_sphere.length as u32, 0, 0);
        }
    }

    pub fn cube<C: Into<Color>>(&self, transform: impl Into<Mat4>, color: C) {
        gpu_event!(self.gpu, "imm_cube");
        let color = color.into();
        self.shader_simple.bind(&self.gpu);

        self.vb_cube.bind_single(&self.gpu);
        self.ib_cube.bind(&self.gpu);

        self.cb_debug_shape
            .write(&ScopeAlkDebugShape {
                local_to_world: transform.into(),
                color: color.0,
            })
            .unwrap();

        self.cb_debug_shape.bind(0, TfxShaderStage::Vertex);
        self.cb_debug_shape.bind(0, TfxShaderStage::Pixel);

        self.gpu.set_input_layout(0);
        self.gpu.set_input_topology(EPrimitiveType::Triangles);
        if color.is_opaque() {
            self.gpu.set_blend_state(0);
        } else {
            self.gpu.set_blend_state(8);
        }

        unsafe {
            self.gpu
                .context()
                .DrawIndexed(self.ib_cube.length as u32, 0, 0);
        }
    }

    pub fn cube_outline<C: Into<Color>>(&self, transform: impl Into<Mat4>, color: C) {
        gpu_event!(self.gpu, "imm_cube_outline");
        let color = color.into();
        self.shader_simple.bind(&self.gpu);

        self.vb_cube.bind_single(&self.gpu);
        self.ib_cube_outline.bind(&self.gpu);

        self.cb_debug_shape
            .write(&ScopeAlkDebugShape {
                local_to_world: transform.into(),
                color: color.0,
            })
            .unwrap();

        self.cb_debug_shape.bind(0, TfxShaderStage::Vertex);
        self.cb_debug_shape.bind(0, TfxShaderStage::Pixel);

        self.gpu.set_input_layout(0);
        self.gpu.set_input_topology(EPrimitiveType::LineList);
        if color.is_opaque() {
            self.gpu.set_blend_state(0);
        } else {
            self.gpu.set_blend_state(8);
        }

        unsafe {
            self.gpu
                .context()
                .DrawIndexed(self.ib_cube_outline.length as u32, 0, 0);
        }
    }
}

// #[derive(Clone)]
// pub enum ShapeFillMode {
//     Solid,
//     Wireframe,
//     SolidWireframe,
// }
//
// #[derive(Clone)]
// pub enum DebugShape {
//     Cube {
//         transform: Transform,
//         fill_mode: ShapeFillMode,
//     },
//     Sphere {
//         center: Vec3,
//         radius: f32,
//     },
//     Line {
//         start: Vec3,
//         end: Vec3,
//     },
//     Circle {
//         center: Vec3,
//         axis: Vec3,
//         edges: u8,
//     },
//     Custom {
//         transform: Transform,
//         shape: CustomDebugShape,
//         sides: bool,
//     },
// }
//
// bitflags! {
//     #[derive(Default, Debug, Copy, Clone, PartialEq)]
//     pub struct DebugDrawFlags: u32 {
//         const DRAW_NORMAL = (1 << 0);
//         const DRAW_PICK = (1 << 1);
//     }
// }
//
// #[derive(Default)]
// pub struct DebugShapes {
//     shapes: Vec<(DebugShape, Color, DebugDrawFlags, Option<Entity>)>,
//     labels: Vec<(String, Vec3, egui::Align2, Color)>,
// }
