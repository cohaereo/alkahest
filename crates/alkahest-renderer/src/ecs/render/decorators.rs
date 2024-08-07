use alkahest_data::{
    decorator::{SDecorator, SUnk80806CB8},
    tfx::{TfxFeatureRenderer, TfxRenderStage, TfxShaderStage},
};
use alkahest_pm::package_manager;
use anyhow::ensure;
use bevy_ecs::component::Component;
use destiny_pkg::TagHash;
use glam::{Mat4, Vec4};
use tiger_parse::PackageManagerExt;

use crate::{
    ecs::render::dynamic_geometry::DynamicModel,
    gpu::{buffer::ConstantBuffer, global_state::RenderStates},
    gpu_event,
    loaders::vertex_buffer::{load_vertex_buffer, VertexBuffer},
    renderer::Renderer,
    tfx::externs,
};

#[derive(Component)]
pub struct DecoratorRenderer {
    pub data: SDecorator,
    pub hash: TagHash,
    pub models: Vec<(
        DynamicModel,
        externs::RigidModel,
        Option<ConstantBuffer<Vec4>>,
    )>,
    instance_buffer: VertexBuffer,
}

impl DecoratorRenderer {
    pub fn load(renderer: &Renderer, hash: TagHash, decorator: SDecorator) -> anyhow::Result<Self> {
        let mut models = vec![];
        for smodel in &decorator.unk8 {
            let mut data = renderer.data.lock();

            let model = DynamicModel::load(
                &mut data.asset_manager,
                smodel.entity_model,
                vec![],
                vec![],
                TfxFeatureRenderer::SpeedtreeTrees,
            )?;
            let ext = externs::RigidModel {
                mesh_to_world: Mat4::IDENTITY,
                position_scale: model.model.model_scale,
                position_offset: model.model.model_offset,
                texcoord0_scale_offset: Vec4::new(
                    model.model.texcoord_scale.x,
                    model.model.texcoord_scale.y,
                    model.model.texcoord_offset.x,
                    model.model.texcoord_offset.y,
                ),
                dynamic_sh_ao_values: Vec4::new(1.0, 1.0, 1.0, 0.0),
            };

            let speedtree_cbuffer = if let Ok(unk34) =
                package_manager().read_tag_struct::<SUnk80806CB8>(smodel.unk34)
            {
                let mut data = vec![Vec4::ONE; 72];
                data[0..5].copy_from_slice(&unk34.unk8[0].unk0);
                Some(ConstantBuffer::create_array_init(
                    renderer.gpu.clone(),
                    &data,
                )?)
            } else {
                None
            };

            models.push((model, ext, speedtree_cbuffer));
        }

        ensure!(!models.is_empty(), "No models found in decorator");

        if models.len() > 1 {
            // anyhow::bail!("Decorators with more than one model are not supported yet");
            warn!("Decorators with more than one model are WIP");
        }

        let instance_buffer = load_vertex_buffer(&renderer.gpu, decorator.unk48.instance_buffer)?;

        Ok(Self {
            models,
            hash,
            data: decorator,
            instance_buffer,
        })
    }

    pub fn draw(&self, renderer: &Renderer, stage: TfxRenderStage) -> anyhow::Result<()> {
        gpu_event!(renderer.gpu, "decorator", self.hash.to_string());

        {
            let mut data = renderer.data.lock();
            let existing_dec = data
                .externs
                .speedtree_placements
                .clone()
                .unwrap_or_default();

            let consts = &self.data.unk48.unk14;
            data.externs.speedtree_placements = Some(externs::SpeedtreePlacements {
                // unk00: consts.instances_offset,
                unk20: consts.instances_scale,
                unk30: consts.instances_offset,
                unk40: consts.unk20,
                unk50: consts.unk30,
                unk60: consts.unk40,
                unk70: consts.unk50,
                // unk40: Default::default(),
                // unk50: Default::default(),
                // unk60: Default::default(),
                ..existing_dec
            });
        }

        for id in 0..(self.data.unk18.len() - 1) {
            let instance_start = self.data.unk18[id];
            let instance_end = self.data.unk18[id + 1];
            let instance_count = instance_end - instance_start;

            let (model, ext, cb) = if self.models.len() == 1 {
                &self.models[0]
            } else {
                &self.models[id]
            };

            if let Some(cb) = cb {
                cb.bind(
                    renderer.render_globals.scopes.speedtree.vertex_slot() as u32,
                    TfxShaderStage::Vertex,
                );
            } else {
                unsafe {
                    renderer
                        .gpu
                        .context()
                        .VSSetConstantBuffers(10, Some(&[None]));
                }
            }

            renderer.data.lock().externs.rigid_model = Some(ext.clone());

            let dyn_id = if self.models.len() == 1 {
                id as u16
            } else {
                // cohae: Multi-models (usually trees) seem to use the ID as a LOD level?
                0
            };

            model.draw_wrapped(
                renderer,
                stage,
                dyn_id,
                move |_model, renderer, mesh, part| unsafe {
                    let layout = mesh.get_input_layout_for_stage(stage);
                    if !RenderStates::is_input_layout_instanced(layout as usize) {
                        // TODO(cohae): Error handling so this doesnt clog the log
                        warn!("Input layout {layout} is not instanced!!");
                        return;
                    }

                    self.instance_buffer.bind_single(&renderer.gpu, 1);

                    renderer.gpu.context().DrawIndexedInstanced(
                        part.index_count,
                        // self.data.unk48.instance_data.data.len() as _,
                        instance_count,
                        part.index_start,
                        0,
                        instance_start,
                    );
                },
            )?;
        }

        Ok(())
    }
}
