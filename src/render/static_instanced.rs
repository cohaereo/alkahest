use crate::entity::VertexBufferHeader;
use crate::map::SMeshInstanceOcclusionBounds;
use crate::packages::package_manager;
use crate::render::scopes::ScopeInstances;
use crate::render::{ConstantBuffer, DeviceContextSwapchain, StaticModel};

use crate::statics::Unk808071a3;
use crate::types::AABB;

use glam::{Mat4, Quat, Vec3};
use hecs::Entity;

use std::sync::Arc;

use super::renderer::Renderer;

pub struct InstancedRenderer {
    renderer: Arc<StaticModel>,
    pub instance_count: usize,
    pub occlusion_bounds: Vec<AABB>,
    instance_buffer: ConstantBuffer<u8>,
}

impl InstancedRenderer {
    pub fn load(
        model: Arc<StaticModel>,
        instances: &[Unk808071a3],
        occlusion_bounds: &[SMeshInstanceOcclusionBounds],
        dcs: Arc<DeviceContextSwapchain>,
    ) -> anyhow::Result<Self> {
        // TODO(cohae): Is this enough to fix it for every buffer set?
        // The last vertex color index, used by the vertex shader to extend the last vertex color value
        let vertex_color_last: Option<usize> = (|| {
            let cbt = model.buffers.first()?.color_buffer;
            let vheader: VertexBufferHeader = package_manager().read_tag_struct(cbt).ok()?;

            Some((vheader.data_size as usize / vheader.stride as usize).saturating_sub(1))
        })();

        let mut instance_data: ScopeInstances = ScopeInstances {
            mesh_offset: model.subheader.mesh_offset.into(),
            mesh_scale: model.subheader.mesh_scale,
            uv_scale: model.subheader.texture_coordinate_scale,
            uv_offset: model.subheader.texture_coordinate_offset.into(),
            max_color_index: vertex_color_last.unwrap_or_default() as u32,

            transforms: Vec::with_capacity(instances.len()),
        };

        for instance in instances {
            let mm = Mat4::from_scale_rotation_translation(
                Vec3::splat(instance.scale.x),
                Quat::from_xyzw(
                    instance.rotation.x,
                    instance.rotation.y,
                    instance.rotation.z,
                    instance.rotation.w,
                )
                .inverse(),
                Vec3::ZERO,
            );

            let model_matrix = Mat4::from_cols(
                mm.x_axis.truncate().extend(instance.translation.x),
                mm.y_axis.truncate().extend(instance.translation.y),
                mm.z_axis.truncate().extend(instance.translation.z),
                mm.w_axis,
            );
            instance_data.transforms.push(model_matrix);

            // let combined_matrix = model.mesh_transform() * model_matrix;

            // let scope_instance = ScopeInstances {
            //     mesh_to_world: combined_matrix.to_3x4(),
            //     texcoord_transform: model.texcoord_transform().extend(f32::from_bits(u32::MAX)),
            // };

            // instance_data.push(scope_instance);
        }

        let instance_buffer = ConstantBuffer::create_array_init(dcs, &instance_data.write())?;

        Ok(Self {
            renderer: model,
            instance_count: instances.len(),
            occlusion_bounds: occlusion_bounds.iter().map(|v| v.bb).collect(),
            instance_buffer,
        })
    }

    pub fn draw(
        &self,
        renderer: &Renderer,
        draw_opaque: bool,
        draw_transparent: bool,
        draw_decals: bool,
        entity: Entity,
    ) -> anyhow::Result<()> {
        self.renderer.draw(
            renderer,
            self.instance_buffer.buffer().clone(),
            self.instance_count,
            draw_opaque,
            draw_transparent,
            draw_decals,
            entity,
        )
    }
}
