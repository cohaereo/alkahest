use alkahest_data::{
    geometry::EPrimitiveType,
    map::STerrain,
    tfx::{TfxRenderStage, TfxShaderStage},
};
use alkahest_pm::package_manager;
use destiny_pkg::TagHash;
use glam::{Mat4, Vec4};
use tiger_parse::PackageManagerExt;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;

use crate::{
    gpu::{buffer::ConstantBuffer, texture::Texture},
    gpu_event,
    handle::Handle,
    loaders::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer},
    renderer::Renderer,
    tfx::technique::Technique,
};

pub struct TerrainPatches {
    terrain: STerrain,
    techniques: Vec<Handle<Technique>>,
    dyemaps: Vec<Handle<Texture>>,
    group_cbuffers: Vec<ConstantBuffer<Mat4>>,

    pub vertex0_buffer: Handle<VertexBuffer>,
    pub vertex1_buffer: Handle<VertexBuffer>,
    pub index_buffer: Handle<IndexBuffer>,

    pub hash: TagHash,
}

impl TerrainPatches {
    pub fn load(renderer: &Renderer, hash: TagHash) -> anyhow::Result<Self> {
        let terrain: STerrain = package_manager().read_tag_struct(hash)?;

        let mut render_data = renderer.data.lock();
        let dyemaps = terrain
            .mesh_groups
            .iter()
            .map(|group| render_data.asset_manager.get_or_load_texture(group.dyemap))
            .collect();
        let techniques = terrain
            .mesh_parts
            .iter()
            .map(|part| {
                render_data
                    .asset_manager
                    .get_or_load_technique(part.technique)
            })
            .collect();

        let mut group_cbuffers = vec![];
        for group in &terrain.mesh_groups {
            let offset = Vec4::new(
                terrain.unk30.x,
                terrain.unk30.y,
                terrain.unk30.z,
                terrain.unk30.w,
            );

            let texcoord_transform =
                Vec4::new(group.unk20.x, group.unk20.y, group.unk20.z, group.unk20.w);

            let scope_terrain = Mat4::from_cols(offset, texcoord_transform, Vec4::ZERO, Vec4::ZERO);

            let cb = ConstantBuffer::create(renderer.gpu.clone(), Some(&scope_terrain))?;
            group_cbuffers.push(cb);
        }

        Ok(Self {
            vertex0_buffer: render_data
                .asset_manager
                .get_or_load_vertex_buffer(terrain.vertex0_buffer),
            vertex1_buffer: render_data
                .asset_manager
                .get_or_load_vertex_buffer(terrain.vertex1_buffer),
            index_buffer: render_data
                .asset_manager
                .get_or_load_index_buffer(terrain.index_buffer),
            terrain,
            techniques,
            dyemaps,
            group_cbuffers,
            hash,
        })
    }

    pub fn draw(&self, renderer: &Renderer, render_stage: TfxRenderStage) {
        if !matches!(
            render_stage,
            TfxRenderStage::GenerateGbuffer
                | TfxRenderStage::ShadowGenerate
                | TfxRenderStage::DepthPrepass
        ) {
            return;
        }

        gpu_event!(renderer.gpu, format!("terrain_patch {}", self.hash));

        // Layout 22
        //  - int4 v0 : POSITION0, // Format DXGI_FORMAT_R16G16B16A16_SINT size 8
        //  - float4 v1 : NORMAL0, // Format DXGI_FORMAT_R16G16B16A16_SNORM size 8
        //  - float2 v2 : TEXCOORD1, // Format DXGI_FORMAT_R16G16_FLOAT size 4
        renderer.gpu.set_input_layout(22);
        renderer
            .gpu
            .set_input_topology(EPrimitiveType::TriangleStrip);

        let vertex0 = renderer
            .data
            .lock()
            .asset_manager
            .vertex_buffers
            .get_shared(&self.vertex0_buffer);
        let vertex1 = renderer
            .data
            .lock()
            .asset_manager
            .vertex_buffers
            .get_shared(&self.vertex1_buffer);
        let index = renderer
            .data
            .lock()
            .asset_manager
            .index_buffers
            .get_shared(&self.index_buffer);

        if let (Some(vertex0), Some(vertex1), Some(index)) = (vertex0, vertex1, index) {
            unsafe {
                let ctx = renderer.gpu.context();
                ctx.IASetIndexBuffer(&index.buffer, DXGI_FORMAT(index.format as _), 0);
                ctx.IASetVertexBuffers(
                    0,
                    2,
                    Some([Some(vertex0.buffer.clone()), Some(vertex1.buffer.clone())].as_ptr()),
                    Some([vertex0.stride as _, vertex1.stride as _].as_ptr()),
                    Some([0, 0].as_ptr()),
                );
            }
        } else {
            return;
        }

        for (i, part) in self
            .terrain
            .mesh_parts
            .iter()
            .enumerate()
            .filter(|(_, u)| u.detail_level == 0)
        {
            let cb11 = &self.group_cbuffers[part.group_index as usize];

            if let Some(technique) = renderer.get_technique_shared(&self.techniques[i]) {
                technique.bind(renderer).expect("Failed to bind technique");
            } else {
                continue;
            }

            cb11.bind(11, TfxShaderStage::Vertex);
            if let Some(dyemap) = renderer
                .data
                .lock()
                .asset_manager
                .textures
                .get(&self.dyemaps[part.group_index as usize])
            {
                dyemap.bind(&renderer.gpu, 14, TfxShaderStage::Pixel);
            }

            unsafe {
                renderer
                    .gpu
                    .context()
                    .DrawIndexed(part.index_count as _, part.index_start as _, 0);
            }
        }
    }
}

pub fn draw_terrain_patches_system(
    renderer: &Renderer,
    scene: &hecs::World,
    render_stage: TfxRenderStage,
) {
    for (e, (terrain,)) in scene.query::<(&TerrainPatches,)>().iter() {
        renderer.pickbuffer.with_entity(e, || {
            terrain.draw(renderer, render_stage);
        });
    }
}
