use std::sync::Arc;

use alkahest_data::tfx::ShaderStage;
use glam::Vec4;

use super::{cbuffer::ConstantBuffer, command_list::CommandList, Gpu};
use crate::gpu_span;

#[repr(C)]
struct ProceduralSpinnerConstants {
    inv_resolution_time: Vec4,
}

pub struct FullscreenSpinner {
    cbuffer: ConstantBuffer<ProceduralSpinnerConstants>,
    start_time: std::time::Instant,

    shader_vs: d3d11::VertexShader,
    shader_ps: d3d11::PixelShader,
}

impl FullscreenSpinner {
    pub fn create(gpu: &Arc<Gpu>) -> anyhow::Result<Self> {
        let cbuffer = ConstantBuffer::create(gpu, None)?;

        let vs_data = include_bytes!("../../builtin/shaders/procedural_spinner.vs.cso");
        let ps_data = include_bytes!("../../builtin/shaders/procedural_spinner.ps.cso");

        let vertex_shader = gpu.create_vertex_shader(vs_data)?;
        let pixel_shader = gpu.create_pixel_shader(ps_data)?;

        Ok(Self {
            cbuffer,
            start_time: std::time::Instant::now(),

            shader_vs: vertex_shader,
            shader_ps: pixel_shader,
        })
    }

    pub fn draw(&self, cmd: &mut CommandList) {
        gpu_span!();
        let time = self.start_time.elapsed().as_secs_f32();
        let (width, height) = cmd.gpu().swapchain_resolution();
        let inv_resolution_time = Vec4::new(1.0 / width as f32, 1.0 / height as f32, time, 0.0);

        self.cbuffer
            .write(
                cmd,
                &ProceduralSpinnerConstants {
                    inv_resolution_time,
                },
            )
            .unwrap();

        self.cbuffer.bind(cmd, ShaderStage::Vertex, 0);

        cmd.vertex_set_shader(Some(&self.shader_vs));
        cmd.pixel_set_shader(Some(&self.shader_ps));
        cmd.input_assembler_set_primitive_topology(d3d11::PrimitiveTopology::TriangleStrip);
        cmd.draw(4, 0);
    }
}
