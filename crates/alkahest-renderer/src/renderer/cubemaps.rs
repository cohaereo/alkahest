use alkahest_data::{geometry::EPrimitiveType, tfx::TfxShaderStage};
use genmesh::{
    generators::{IndexedPolygon, SharedVertex},
    Triangulate,
};
use glam::Mat4;
use windows::Win32::Graphics::Direct3D11::{ID3D11PixelShader, ID3D11VertexShader};

use crate::{
    ecs::{map::CubemapVolume, transform::Transform, Scene},
    gpu::{buffer::ConstantBuffer, util::DxDeviceExt, SharedGpuContext},
    include_dxbc,
    loaders::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer},
    renderer::Renderer,
    tfx::{externs, globals::CubemapShape},
};

pub fn draw_cubemap_system(renderer: &Renderer, scene: &mut Scene) {
    {
        renderer.data.lock().externs.cubemaps = Some(externs::Cubemaps {
            temp_ao: renderer.gpu.white_texture.view.clone().into(),
        });
    }

    for (transform, cubemap) in scene.query::<(&Transform, &CubemapVolume)>().iter(scene) {
        let alpha = false;
        let probes = cubemap.voxel_diffuse.is_some();
        let relighting = false;

        let pipeline = renderer
            .render_globals
            .pipelines
            .get_specialized_cubemap_pipeline(CubemapShape::Cube, alpha, probes, relighting);

        if let Err(e) = pipeline.bind(renderer) {
            error!(
                "Failed to run cubemap pipeline (alpha={alpha}, probes={probes}, \
                 relighting={relighting}): {e:?}"
            );
            return;
        }

        renderer.cubemap_renderer.draw(renderer, transform, cubemap);
    }
}

pub struct CubemapRenderer {
    shader_vs: ID3D11VertexShader,
    shader_ps: ID3D11PixelShader,

    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,

    cbuffer: ConstantBuffer<(Mat4, Mat4, Mat4)>,
}

impl CubemapRenderer {
    pub fn new(gpu: SharedGpuContext) -> anyhow::Result<Self> {
        let shader_vs = gpu
            .device
            .load_vertex_shader(include_dxbc!(vs "cubemap.hlsl"))
            .unwrap();
        let shader_ps = gpu
            .device
            .load_pixel_shader(include_dxbc!(ps "cubemap.hlsl"))
            .unwrap();

        let mesh = genmesh::generators::Cube::new();
        let vertices: Vec<[f32; 4]> = mesh
            .shared_vertex_iter()
            .map(|v| {
                let v = <[f32; 3]>::from(v.pos);
                [v[0], v[1], v[2], 1.0]
            })
            .collect();
        let mut indices = vec![];
        for i in mesh.indexed_polygon_iter().triangulate() {
            indices.extend_from_slice(&[i.x as u16, i.y as u16, i.z as u16]);
        }

        let index_buffer = IndexBuffer::load_u16(&gpu, &indices)?;
        let vertex_buffer = VertexBuffer::load_data(
            &gpu.device,
            bytemuck::cast_slice(&vertices),
            std::mem::size_of::<[f32; 4]>() as u32,
        )?;

        Ok(Self {
            shader_vs,
            shader_ps,
            vertex_buffer,
            index_buffer,
            cbuffer: ConstantBuffer::create(gpu.clone(), None)?,
        })
    }

    pub fn draw(&self, renderer: &Renderer, transform: &Transform, cubemap: &CubemapVolume) {
        self.vertex_buffer.bind_single(&renderer.gpu, 0);
        self.index_buffer.bind(&renderer.gpu);

        let matrix = Mat4::from_scale_rotation_translation(
            cubemap.extents * -1.0,
            transform.rotation,
            transform.translation,
        );
        let target_pixel_to_world = renderer
            .data
            .lock()
            .externs
            .view
            .as_ref()
            .map(|v| v.target_pixel_to_world)
            .unwrap_or_default();

        self.cbuffer
            .write(&(matrix, matrix.inverse(), target_pixel_to_world))
            .unwrap();
        self.cbuffer.bind(0, TfxShaderStage::Vertex);
        self.cbuffer.bind(0, TfxShaderStage::Pixel);

        renderer.gpu.flush_states();

        unsafe {
            {
                let data = renderer.data.lock();
                let specular_ibl = data
                    .asset_manager
                    .textures
                    .get(&cubemap.specular_ibl)
                    .map(|v| v.view.clone());
                let diffuse_ibl = cubemap
                    .voxel_diffuse
                    .as_ref()
                    .and_then(|t| data.asset_manager.textures.get(t).map(|v| v.view.clone()));

                renderer.gpu.lock_context().PSSetShaderResources(
                    0,
                    Some(&[
                        specular_ibl,
                        diffuse_ibl,
                        Some(data.gbuffers.rt1_read.view.clone()),
                        Some(data.gbuffers.depth.texture_view.clone()),
                    ]),
                );
            }

            renderer
                .gpu
                .lock_context()
                .VSSetShader(&self.shader_vs, None);
            renderer
                .gpu
                .lock_context()
                .PSSetShader(&self.shader_ps, None);
            renderer.gpu.set_input_layout(0);
            renderer.gpu.set_input_topology(EPrimitiveType::Triangles);

            renderer
                .gpu
                .lock_context()
                .DrawIndexed(self.index_buffer.length as u32, 0, 0);
        }
    }
}
