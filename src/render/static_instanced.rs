use crate::render::scopes::ScopeInstances;
use crate::render::{ConstantBuffer, DeviceContextSwapchain, StaticModel};

use crate::statics::Unk808071a3;

use glam::{Mat4, Quat, Vec3};

use std::rc::Rc;
use std::sync::Arc;

use super::scopes::MatrixConversion;
use super::RenderData;

pub struct InstancedRenderer {
    renderer: Arc<StaticModel>,
    instance_count: usize,
    instance_buffer: ConstantBuffer<ScopeInstances>,
}

impl InstancedRenderer {
    pub fn load(
        model: Arc<StaticModel>,
        instances: &[Unk808071a3],
        dcs: Rc<DeviceContextSwapchain>,
    ) -> anyhow::Result<Self> {
        let mut instance_data: Vec<ScopeInstances> = Vec::with_capacity(instances.len());

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

            let combined_matrix = model.mesh_transform() * model_matrix;

            let scope_instance = ScopeInstances {
                mesh_to_world: combined_matrix.to_3x4(),
                texcoord_transform: model.texcoord_transform().extend(f32::from_bits(u32::MAX)),
            };

            instance_data.push(scope_instance);
        }

        let instance_buffer = ConstantBuffer::create_array_init(dcs, &instance_data)?;

        Ok(Self {
            renderer: model,
            instance_count: instance_data.len(),
            instance_buffer,
        })
    }

    pub fn draw(
        &self,
        dcs: &DeviceContextSwapchain,
        render_data: &RenderData,
        draw_transparent: bool,
    ) -> anyhow::Result<()> {
        unsafe {
            dcs.context
                .VSSetConstantBuffers(11, Some(&[Some(self.instance_buffer.buffer().clone())]));
        }

        self.renderer
            .draw(dcs, render_data, self.instance_count, draw_transparent)
    }
}
