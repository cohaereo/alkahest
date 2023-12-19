use windows::Win32::Graphics::{
    Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
    Direct3D11::{ID3D11PixelShader, ID3D11VertexShader},
};

use super::{shader, DeviceContextSwapchain};

pub struct OutlineScreenEffect {
    outline_vs: ID3D11VertexShader,
    outline_ps: ID3D11PixelShader,
}

impl OutlineScreenEffect {
    pub fn create(dcs: &DeviceContextSwapchain) -> anyhow::Result<Self> {
        let vshader_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/outline.hlsl"),
            "VShader",
            "vs_5_0",
        )
        .unwrap();
        let pshader_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/outline.hlsl"),
            "PShader",
            "ps_5_0",
        )
        .unwrap();

        let (outline_vs, _) = shader::load_vshader(dcs, &vshader_blob)?;
        let (outline_ps, _) = shader::load_pshader(dcs, &pshader_blob)?;

        Ok(Self {
            outline_vs,
            outline_ps,
        })
    }

    pub fn draw(&self, dcs: &DeviceContextSwapchain) {
        unsafe {
            dcs.context().VSSetShader(&self.outline_vs, None);
            dcs.context().PSSetShader(&self.outline_ps, None);
            dcs.context()
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            dcs.context().Draw(4, 0);

            dcs.context()
                .PSSetShaderResources(0, Some(&[None, None, None, None, None]));
        }
    }
}
