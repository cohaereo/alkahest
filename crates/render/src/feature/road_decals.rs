use alkahest_data::tfx::{
    ShaderStage,
    features::{
        dynamic::RenderStageSubscription,
        road_decals::{SRoadDecal, SRoadDecalCollection},
    },
};
use glam::{Mat4, Vec4Swizzles, vec4};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use super::FeatureRenderer;
use crate::{
    Renderer,
    asset::{Handle, index_buffer::IndexBuffer, vertex_buffer::VertexBuffer},
    feature::rigid_model::RigidModelConstants,
    gpu::{cbuffer::ConstantBuffer, command_list::CommandList},
    tfx::technique::Technique,
};

pub struct RoadDecalCollectionRenderer {
    decals: Vec<RoadDecal>,
}

pub struct RoadDecal {
    pub data: SRoadDecal,
    pub technique: Handle<Technique>,
    pub vertex_buffer: Handle<VertexBuffer>,
    pub index_buffer: Handle<IndexBuffer>,
    pub cb: ConstantBuffer<RigidModelConstants>,
}

impl RoadDecalCollectionRenderer {
    #[profiling::function]
    pub fn load(hash: TagHash) -> anyhow::Result<Box<Self>> {
        let collection = package_manager().read_tag_struct::<SRoadDecalCollection>(hash)?;

        Ok(Box::new(Self {
            decals: collection
                .decals
                .into_iter()
                .map(|decal| {
                    let cb = ConstantBuffer::create(
                        &Renderer::instance().gpu,
                        Some(&RigidModelConstants {
                            mesh_to_world: Mat4::from_rotation_translation(
                                decal.rotation,
                                decal.position.xyz(),
                            ),
                            position_scale: decal.model_scale,
                            position_offset: decal.model_offset,
                            texcoord0_scale_offset: vec4(
                                decal.texcoord_scale.x,
                                decal.texcoord_scale.y,
                                decal.texcoord_offset.x,
                                decal.texcoord_offset.y,
                            ),
                            dynamic_sh_ao_values: vec4(0.0, 0.0, 0.8, 0.8),
                        }),
                    )
                    .expect("Failed to create constant buffer for road decal");

                    RoadDecal {
                        technique: Renderer::instance().asset_manager.load(decal.technique),
                        vertex_buffer: Renderer::instance().asset_manager.load(decal.vertex_buffer),
                        index_buffer: Renderer::instance().asset_manager.load(decal.index_buffer),
                        data: decal,
                        cb,
                    }
                })
                .collect(),
        }))
    }
}

#[profiling::all_functions]
impl FeatureRenderer for RoadDecalCollectionRenderer {
    fn extract_and_prepare(&mut self, _renderer: &Renderer, _extracted_data: &dyn std::any::Any) {}

    fn submit(&self, cmd: &mut CommandList, _stage: alkahest_data::tfx::RenderStage) {
        for decal in &self.decals {
            let Some((vb, ib)) = decal.vertex_buffer.get().zip(decal.index_buffer.get()) else {
                return;
            };

            decal.cb.bind(cmd, ShaderStage::Vertex, 1);
            vb.bind_single(cmd, 0);
            ib.bind(cmd);

            cmd.set_input_layout(9);
            cmd.set_input_topology(alkahest_data::tfx::PrimitiveType::Triangles);
            let Some(t) = decal.technique.get() else {
                continue;
            };
            t.bind(cmd).unwrap();
            cmd.draw_indexed(
                decal.data.face_count as u32 * 3,
                decal.data.index_start as u32,
                0,
            );
        }
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        RenderStageSubscription::DECALS
    }
}
