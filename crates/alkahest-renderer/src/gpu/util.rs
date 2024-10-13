use alkahest_data::geometry::EPrimitiveType;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11ComputeShader, ID3D11Device, ID3D11PixelShader, ID3D11RenderTargetView,
    ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D, ID3D11VertexShader,
    D3D11_COMPARISON_NEVER, D3D11_FILTER_MIN_MAG_MIP_POINT, D3D11_SAMPLER_DESC,
    D3D11_TEXTURE_ADDRESS_CLAMP,
};

use crate::{gpu::GpuContext, gpu_event, include_dxbc};

pub struct UtilResources {
    pub entity_vs_override: ID3D11VertexShader,
    pub blit_vs: ID3D11VertexShader,
    pub blit_ps: ID3D11PixelShader,
    pub blit_srgb_ps: ID3D11PixelShader,
    pub blit_alphaluminance_ps: ID3D11PixelShader,

    pub point_sampler: ID3D11SamplerState,
}

impl UtilResources {
    pub fn new(device: &ID3D11Device) -> Self {
        let entity_vs_override = device
            .load_vertex_shader(include_dxbc!(vs "debug/entity_vs_override.hlsl"))
            .unwrap();
        let blit_vs = device
            .load_vertex_shader(include_dxbc!(vs "util/blit.hlsl"))
            .unwrap();
        let blit_ps = device
            .load_pixel_shader(include_dxbc!(ps "util/blit.hlsl"))
            .unwrap();
        let blit_srgb_ps = device
            .load_pixel_shader(include_dxbc!(ps "util/blit_srgb.hlsl"))
            .unwrap();
        let blit_alphaluminance_ps = device
            .load_pixel_shader(include_dxbc!(ps "util/copy_with_luminance_as_alpha.hlsl"))
            .unwrap();

        let point_sampler = device
            .create_sampler_state(&D3D11_SAMPLER_DESC {
                Filter: D3D11_FILTER_MIN_MAG_MIP_POINT,
                AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                MipLODBias: 0.0,
                MaxAnisotropy: 1,
                ComparisonFunc: D3D11_COMPARISON_NEVER,
                BorderColor: [0.0, 0.0, 0.0, 0.0],
                MinLOD: 0.0,
                MaxLOD: f32::MAX,
            })
            .unwrap();

        Self {
            entity_vs_override,
            blit_vs,
            blit_ps,
            blit_srgb_ps,
            blit_alphaluminance_ps,
            point_sampler,
        }
    }
}

impl GpuContext {
    pub fn blit_texture(
        &self,
        texture_view: &ID3D11ShaderResourceView,
        rt: &ID3D11RenderTargetView,
        srgb: bool,
    ) {
        gpu_event!(self, "blit_texture");
        self.blit_internal(
            texture_view,
            rt,
            if srgb {
                &self.util_resources.blit_srgb_ps
            } else {
                &self.util_resources.blit_ps
            },
        );
    }

    pub fn blit_texture_alphaluminance(
        &self,
        texture_view: &ID3D11ShaderResourceView,
        rt: &ID3D11RenderTargetView,
    ) {
        gpu_event!(self, "blit_texture_alphaluminance");
        self.blit_internal(
            texture_view,
            rt,
            &self.util_resources.blit_alphaluminance_ps,
        );
    }

    fn blit_internal(
        &self,
        texture_view: &ID3D11ShaderResourceView,
        rt: &ID3D11RenderTargetView,
        shader: &ID3D11PixelShader,
    ) {
        unsafe {
            self.set_blend_state(0);
            // self.set_rasterizer_state(0);
            self.context.RSSetState(None);

            self.context.VSSetShader(&self.util_resources.blit_vs, None);
            self.context.PSSetShader(shader, None);

            self.set_input_topology(EPrimitiveType::Triangles);
            self.context
                .OMSetRenderTargets(Some(&[Some(rt.clone())]), None);
            self.context.OMSetDepthStencilState(None, 0);
            self.context
                .PSSetSamplers(0, Some(&[Some(self.util_resources.point_sampler.clone())]));
            self.context
                .PSSetShaderResources(0, Some(&[Some(texture_view.clone())]));

            self.context.Draw(3, 0);
        }
    }

    pub fn copy_texture(&self, source: &ID3D11Texture2D, dest: &ID3D11Texture2D) {
        unsafe {
            self.context().CopyResource(dest, source);
        }
    }
}

pub trait DxDeviceExt {
    fn load_vertex_shader(&self, data: &[u8]) -> anyhow::Result<ID3D11VertexShader>;
    fn load_pixel_shader(&self, data: &[u8]) -> anyhow::Result<ID3D11PixelShader>;
    fn load_compute_shader(&self, data: &[u8]) -> anyhow::Result<ID3D11ComputeShader>;

    fn create_sampler_state(&self, desc: &D3D11_SAMPLER_DESC)
        -> anyhow::Result<ID3D11SamplerState>;
}

impl DxDeviceExt for ID3D11Device {
    fn load_vertex_shader(&self, data: &[u8]) -> anyhow::Result<ID3D11VertexShader> {
        let mut shader = None;
        unsafe { self.CreateVertexShader(data, None, Some(&mut shader)) }.unwrap();
        Ok(shader.unwrap())
    }

    fn load_pixel_shader(&self, data: &[u8]) -> anyhow::Result<ID3D11PixelShader> {
        let mut shader = None;
        unsafe { self.CreatePixelShader(data, None, Some(&mut shader)) }.unwrap();
        Ok(shader.unwrap())
    }

    fn load_compute_shader(&self, data: &[u8]) -> anyhow::Result<ID3D11ComputeShader> {
        let mut shader = None;
        unsafe { self.CreateComputeShader(data, None, Some(&mut shader)) }.unwrap();
        Ok(shader.unwrap())
    }

    fn create_sampler_state(
        &self,
        desc: &D3D11_SAMPLER_DESC,
    ) -> anyhow::Result<ID3D11SamplerState> {
        let mut sampler = None;
        unsafe { self.CreateSamplerState(desc, Some(&mut sampler)) }.unwrap();
        Ok(sampler.unwrap())
    }
}
