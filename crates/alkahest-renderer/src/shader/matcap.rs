use alkahest_data::{geometry::EPrimitiveType, tfx::TfxShaderStage};
use glam::Mat4;
use windows::Win32::Graphics::Direct3D11::{ID3D11PixelShader, ID3D11VertexShader};

use crate::{
    gpu::{
        buffer::ConstantBuffer, texture::Texture, util::DxDeviceExt, GpuContext, SharedGpuContext,
    },
    include_dxbc,
    tfx::{externs::ExternStorage, gbuffer::GBuffer},
    util::image::Png,
};

/// Renders a matcap texture onto the light buffers
pub struct MatcapRenderer {
    shader_vs: ID3D11VertexShader,
    shader_ps: ID3D11PixelShader,

    cam_cb: ConstantBuffer<Mat4>,
    matcap_diffuse: Texture,
    matcap_specular: Texture,
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

        Ok(Self {
            shader_vs,
            shader_ps,
            cam_cb,
            matcap_diffuse,
            matcap_specular,
        })
    }

    pub fn draw(&self, gctx: &GpuContext, externs: &ExternStorage, gbuffers: &GBuffer) {
        unsafe {
            if let Some(view) = &externs.view {
                self.cam_cb.write(&view.world_to_camera).unwrap();
            } else {
                return;
            }

            self.cam_cb.bind(0, TfxShaderStage::Pixel);
            gctx.context()
                .PSSetShaderResources(0, Some(&[Some(gbuffers.rt1.view.clone())]));

            self.matcap_diffuse.bind(gctx, 1, TfxShaderStage::Pixel);
            self.matcap_specular.bind(gctx, 2, TfxShaderStage::Pixel);

            gctx.context().RSSetState(None);
            gctx.set_input_topology(EPrimitiveType::Triangles);
            gctx.context().OMSetDepthStencilState(None, 0);
            gctx.context().VSSetShader(&self.shader_vs, None);
            gctx.context().PSSetShader(&self.shader_ps, None);

            gctx.context().Draw(3, 0);
        }
    }
}
