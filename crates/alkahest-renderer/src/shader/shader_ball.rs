use std::io::Cursor;

use alkahest_data::{geometry::EPrimitiveType, technique::StateSelection, tfx::TfxShaderStage};
use glam::{Mat4, Vec3, Vec4};
use windows::Win32::Graphics::Direct3D11::{ID3D11PixelShader, ID3D11VertexShader};

use crate::{
    ecs::{transform::Transform, Scene},
    gpu::{buffer::ConstantBuffer, util::DxDeviceExt},
    gpu_event, include_dxbc,
    loaders::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer},
    renderer::Renderer,
    tfx::technique::ShaderModule,
};

#[repr(C)]
struct ShaderBallCbuffer {
    model_to_world: Mat4,
    rgb_iridescent: Vec4,
    smoothness: f32,
    metalness: f32,
    emission: f32,
    transmission: f32,
}

pub struct ShaderBallRenderer {
    vertex_buffer: VertexBuffer,
    // index_buffer: IndexBuffer,
    vshader: ID3D11VertexShader,
    pshader: ID3D11PixelShader,

    cbuffer: ConstantBuffer<ShaderBallCbuffer>,
}

impl ShaderBallRenderer {
    pub fn new(renderer: &Renderer) -> anyhow::Result<Self> {
        let obj_data = include_bytes!("../../assets/models/shaderball.obj");
        let reader = Cursor::new(obj_data);
        let obj = obj::ObjData::load_buf(reader).unwrap();

        let mut vertices = vec![];
        for vb in &obj.objects[0].groups[0].polys {
            for p in &vb.0 {
                let vi = p.0;
                let vti = p.1.unwrap_or_default();
                let vni = p.2.unwrap_or_default();

                vertices.push([
                    obj.position[vi][0],
                    obj.position[vi][1],
                    obj.position[vi][2],
                    obj.texture[vti][0],
                    obj.texture[vti][1],
                    obj.normal[vni][0],
                    obj.normal[vni][1],
                    obj.normal[vni][2],
                ]);
            }
        }

        let vshader = renderer
            .gpu
            .device
            .load_vertex_shader(include_dxbc!(vs "misc/gbuffer_test.hlsl"))?;
        let pshader = renderer
            .gpu
            .device
            .load_pixel_shader(include_dxbc!(ps "misc/gbuffer_test.hlsl"))?;

        Ok(Self {
            vertex_buffer: VertexBuffer::load_data(
                &renderer.gpu.device,
                bytemuck::cast_slice(&vertices),
                32,
            )?,
            vshader,
            pshader,
            cbuffer: ConstantBuffer::create(renderer.gpu.clone(), None)?,
        })
    }
}

pub struct ShaderBallComponent {
    renderer: ShaderBallRenderer,

    pub color: Vec3,
    pub iridescence: u32,
    pub smoothness: f32,
    pub metalness: f32,
    pub emission: f32,
    pub transmission: f32,
}

impl ShaderBallComponent {
    pub fn new(renderer: &Renderer) -> anyhow::Result<Self> {
        Ok(Self {
            renderer: ShaderBallRenderer::new(renderer)?,
            color: Vec3::ONE,
            iridescence: 0,
            smoothness: 0.5,
            metalness: 0.0,
            emission: 0.0,
            transmission: 0.0,
        })
    }
}

pub fn draw_shaderball_system(renderer: &Renderer, scene: &Scene) {
    for (_, (transform, ball)) in scene.query::<(&Transform, &ShaderBallComponent)>().iter() {
        gpu_event!(renderer.gpu, "draw_shaderball");
        ball.renderer
            .cbuffer
            .write(&ShaderBallCbuffer {
                model_to_world: transform.local_to_world(),
                rgb_iridescent: ball.color.extend(ball.iridescence as f32 / 128.0),
                smoothness: ball.smoothness,
                metalness: ball.metalness,
                emission: ball.emission,
                transmission: ball.transmission,
            })
            .unwrap();

        renderer.gpu.set_input_topology(EPrimitiveType::Triangles);
        renderer.gpu.set_input_layout(12);
        ball.renderer.vertex_buffer.bind_single(&renderer.gpu, 0);
        ball.renderer.cbuffer.bind(0, TfxShaderStage::Vertex);
        ball.renderer.cbuffer.bind(0, TfxShaderStage::Pixel);

        renderer.gpu.set_blend_state(0);
        renderer.gpu.set_depth_stencil_state(2);
        renderer.gpu.set_rasterizer_state(2);
        renderer.gpu.set_depth_bias(0);

        unsafe {
            renderer
                .gpu
                .context()
                .VSSetShader(&ball.renderer.vshader, None);
            renderer
                .gpu
                .context()
                .PSSetShader(&ball.renderer.pshader, None);

            renderer
                .gpu
                .context()
                .Draw(ball.renderer.vertex_buffer.length, 0);
        }
    }
}
