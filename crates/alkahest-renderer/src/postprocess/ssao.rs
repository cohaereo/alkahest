use alkahest_data::{
    dxgi::DxgiFormat, geometry::EPrimitiveType, technique::StateSelection, tfx::TfxShaderStage,
};
use glam::{FloatExt, Mat4, Vec3, Vec4};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11PixelShader, ID3D11RenderTargetView, ID3D11VertexShader,
};

use crate::{
    gpu::{
        buffer::ConstantBufferCached, texture::Texture, util::DxDeviceExt, GpuContext,
        SharedGpuContext,
    },
    include_dxbc,
    tfx::{externs::ExternStorage, gbuffer::RenderTarget},
};

pub struct SsaoRenderer {
    pub scope: ConstantBufferCached<ScopeAlkahestSsao>,
    noise_texture: Texture,

    shader_vs: ID3D11VertexShader,
    shader_ps: ID3D11PixelShader,
    shader_blur_ps: ID3D11PixelShader,
}

impl SsaoRenderer {
    pub fn new(gctx: SharedGpuContext) -> anyhow::Result<Self> {
        let mut noise = vec![];
        let mut rng = fastrand::Rng::with_seed(0xcb65a5a72901bc71);
        for _ in 0..16 {
            noise.push(Vec3::new(rng.f32() * 2.0 - 1.0, rng.f32() * 2.0 - 1.0, 0.0));
        }

        let noise_texture = Texture::load_2d_raw(
            &gctx.device,
            4,
            4,
            bytemuck::cast_slice(&noise),
            DxgiFormat::R32G32B32_FLOAT,
            Some("SSAO Noise Texture"),
        )?;

        let shader_vs = gctx
            .device
            .load_vertex_shader(include_dxbc!(vs "postprocess/ssao.hlsl"))
            .unwrap();
        let shader_ps = gctx
            .device
            .load_pixel_shader(include_dxbc!(ps "postprocess/ssao.hlsl"))
            .unwrap();
        let shader_blur_ps = gctx
            .device
            .load_pixel_shader(include_dxbc!(ps "postprocess/ssao_blur_and_apply.hlsl"))
            .unwrap();

        Ok(Self {
            scope: ConstantBufferCached::create_init(gctx.clone(), &ScopeAlkahestSsao::default())?,
            noise_texture,
            shader_vs,
            shader_ps,
            shader_blur_ps,
        })
    }

    pub fn draw(&self, gctx: &GpuContext, externs: &ExternStorage, intermediate_rt: &RenderTarget) {
        {
            let scope = self.scope.data();
            if let Some(view) = &externs.view {
                scope.target_pixel_to_world = view.target_pixel_to_world
            } else {
                return;
            }
        }

        if let Some(deferred) = &externs.deferred {
            unsafe {
                gctx.context().PSSetShaderResources(
                    0,
                    Some(&[deferred.deferred_depth.view(), deferred.deferred_rt1.view()]),
                );

                gctx.context()
                    .PSSetConstantBuffers(0, Some(&[Some(self.scope.buffer().clone())]));
            }

            self.noise_texture.bind(gctx, 2, TfxShaderStage::Pixel);
        } else {
            return;
        }

        unsafe {
            const NO_RT: Option<ID3D11RenderTargetView> = None;
            let mut rt_backup = [NO_RT; 4];
            gctx.context()
                .OMGetRenderTargets(Some(&mut rt_backup), None);

            gctx.context()
                .OMSetRenderTargets(Some(&[Some(intermediate_rt.render_target.clone())]), None);

            gctx.set_blend_state(0);
            gctx.context().RSSetState(None);
            gctx.set_input_topology(EPrimitiveType::Triangles);
            gctx.context().OMSetDepthStencilState(None, 0);
            gctx.context().VSSetShader(&self.shader_vs, None);
            gctx.context().PSSetShader(&self.shader_ps, None);

            gctx.context().Draw(3, 0);

            gctx.current_states
                .store(StateSelection::new(Some(3), Some(1), Some(1), Some(1)));
            gctx.set_blend_state(3);
            gctx.context().OMSetRenderTargets(Some(&rt_backup), None);
            gctx.context().PSSetShader(&self.shader_blur_ps, None);
            gctx.context()
                .PSSetShaderResources(0, Some(&[Some(intermediate_rt.view.clone())]));
            gctx.context().Draw(3, 0);
        }
    }
}

const KERNEL_SIZE: usize = 16;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ScopeAlkahestSsao {
    pub target_pixel_to_world: Mat4,

    pub radius: f32,
    pub bias: f32,
    pub kernel_size: i32,
    pub samples: [Vec4; KERNEL_SIZE],
}

impl Default for ScopeAlkahestSsao {
    fn default() -> Self {
        // TODO(cohae): Configurable kernel size
        let mut samples = [Vec4::ZERO; KERNEL_SIZE];
        let mut rng = fastrand::Rng::with_seed(0xbc65a5a72901bc71);
        for i in 0..KERNEL_SIZE {
            let mut sample = Vec3::new(rng.f32() * 2.0 - 1.0, rng.f32() * 2.0 - 1.0, 0.0);
            sample = sample.normalize();
            sample *= rng.f32();

            let mut scale = i as f32 / KERNEL_SIZE as f32;
            scale = 0.1f32.lerp(1.0, scale * scale);
            sample *= scale;
            samples[i] = sample.extend(1.0);
        }

        Self {
            target_pixel_to_world: Default::default(),
            radius: 3.0,
            bias: 0.15,
            kernel_size: KERNEL_SIZE as _,
            samples,
        }
    }
}
