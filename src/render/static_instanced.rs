use crate::material;
use crate::render::scopes::ScopeStaticInstance;
use crate::render::static_render::LoadedTexture;
use crate::render::{DeviceContextSwapchain, StaticModel};
use crate::statics::Unk808071a3;
use glam::{Mat4, Quat, Vec3};
use nohash_hasher::IntMap;
use std::sync::Arc;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11DeviceContext, ID3D11InputLayout, ID3D11PixelShader, ID3D11SamplerState,
    ID3D11VertexShader, D3D11_BIND_CONSTANT_BUFFER, D3D11_BUFFER_DESC, D3D11_CPU_ACCESS_WRITE,
    D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DYNAMIC,
};

pub struct InstancedRenderer {
    renderer: Arc<StaticModel>,
    instance_count: usize,
    instance_buffer: ID3D11Buffer,
}

impl InstancedRenderer {
    pub fn load(
        model: Arc<StaticModel>,
        instances: &[Unk808071a3],
        dcs: &DeviceContextSwapchain,
    ) -> anyhow::Result<Self> {
        let mut instance_data: Vec<ScopeStaticInstance> = Vec::with_capacity(instances.len());

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

            let scope_instance = ScopeStaticInstance {
                mesh_to_world: Mat4 {
                    x_axis: combined_matrix.x_axis,
                    y_axis: combined_matrix.y_axis,
                    z_axis: combined_matrix.z_axis,
                    w_axis: model.texcoord_transform().extend(f32::from_bits(u32::MAX)),
                },
            };

            instance_data.push(scope_instance);
        }

        let instance_buffer = unsafe {
            dcs.device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    Usage: D3D11_USAGE_DYNAMIC,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                    CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                    ByteWidth: (std::mem::size_of::<ScopeStaticInstance>() * instance_data.len())
                        as _,
                    ..Default::default()
                },
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: instance_data.as_ptr() as _,
                    SysMemPitch: std::mem::size_of::<ScopeStaticInstance>() as _,
                    ..Default::default()
                }),
            )?
        };

        Ok(Self {
            renderer: model,
            instance_count: instance_data.len(),
            instance_buffer,
        })
    }

    pub fn draw(
        &self,
        device_context: &ID3D11DeviceContext,
        materials: &IntMap<u32, material::Unk808071e8>,
        vshaders: &IntMap<u32, (ID3D11VertexShader, ID3D11InputLayout)>,
        pshaders: &IntMap<u32, ID3D11PixelShader>,
        cbuffers_vs: &IntMap<u32, ID3D11Buffer>,
        cbuffers_ps: &IntMap<u32, ID3D11Buffer>,
        textures: &IntMap<u32, LoadedTexture>,
        samplers: &IntMap<u32, ID3D11SamplerState>,
        cbuffer_default: ID3D11Buffer,
    ) {
        unsafe {
            device_context.VSSetConstantBuffers(11, Some(&[Some(self.instance_buffer.clone())]));
        }

        self.renderer.draw(
            device_context,
            materials,
            vshaders,
            pshaders,
            cbuffers_vs,
            cbuffers_ps,
            textures,
            samplers,
            cbuffer_default,
            self.instance_count,
        );
    }
}
