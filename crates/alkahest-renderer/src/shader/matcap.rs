use alkahest_data::{geometry::EPrimitiveType, tfx::TfxShaderStage};
use glam::Mat4;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11PixelShader, ID3D11SamplerState, ID3D11VertexShader, D3D11_FILTER_MIN_MAG_MIP_LINEAR,
    D3D11_SAMPLER_DESC, D3D11_TEXTURE_ADDRESS_CLAMP,
};

use crate::{
    gpu::{buffer::ConstantBuffer, texture::Texture, util::DxDeviceExt, SharedGpuContext},
    include_dxbc,
    renderer::Renderer,
    util::image::Png,
};

/// Renders a matcap texture onto the light buffers
pub struct MatcapRenderer {
    shader_vs: ID3D11VertexShader,
    shader_ps: ID3D11PixelShader,

    cam_cb: ConstantBuffer<Mat4>,
    matcap_diffuse: Texture,
    matcap_specular: Texture,
    sampler_linear: ID3D11SamplerState,
}

impl MatcapRenderer {
    pub fn new(gctx: SharedGpuContext) -> anyhow::Result<Self> {
        let cam_cb = ConstantBuffer::create(gctx.clone(), None)?;

        let shader_vs = gctx
            .device
            .load_vertex_shader(include_dxbc!(vs "misc/matcap.hlsl"))
            .unwrap();
        let shader_ps = gctx
            .device
            .load_pixel_shader(include_dxbc!(ps "misc/matcap.hlsl"))
            .unwrap();

        let matcap_diffuse = Texture::load_png(
            &gctx.device,
            &Png::from_bytes(include_bytes!("../../assets/textures/matcap.png"))?,
            Some("matcap.png"),
        )?;

        let matcap_specular = Texture::load_png(
            &gctx.device,
            &Png::from_bytes(include_bytes!("../../assets/textures/matcap_specular.png"))?,
            Some("matcap_specular.png"),
        )?;

        let sampler_linear = gctx.device.create_sampler_state(&D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            ..Default::default()
        })?;

        Ok(Self {
            shader_vs,
            shader_ps,
            cam_cb,
            matcap_diffuse,
            matcap_specular,
            sampler_linear,
        })
    }

    pub fn draw(&self, renderer: &Renderer) {
        unsafe {
            let data = renderer.data.lock();
            if let Some(view) = &data.externs.view {
                self.cam_cb.write(&view.world_to_camera).unwrap();
            } else {
                return;
            }

            self.cam_cb.bind(0, TfxShaderStage::Pixel);
            renderer
                .gpu
                .lock_context()
                .PSSetShaderResources(0, Some(&[Some(data.gbuffers.rt1.view.clone())]));
            renderer
                .gpu
                .lock_context()
                .PSSetSamplers(0, Some(&[Some(self.sampler_linear.clone())]));

            self.matcap_diffuse
                .bind(&renderer.gpu, 1, TfxShaderStage::Pixel);
            self.matcap_specular
                .bind(&renderer.gpu, 2, TfxShaderStage::Pixel);

            renderer.gpu.flush_states();
            renderer.gpu.lock_context().RSSetState(None);
            renderer.gpu.set_input_topology(EPrimitiveType::Triangles);
            renderer.gpu.lock_context().OMSetDepthStencilState(None, 0);
            renderer.gpu.lock_context().VSSetShader(&self.shader_vs, None);
            renderer.gpu.lock_context().PSSetShader(&self.shader_ps, None);

            renderer.gpu.lock_context().Draw(3, 0);
        }
    }
}
