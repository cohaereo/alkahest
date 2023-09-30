use windows::Win32::Graphics::Direct3D11::{ID3D11PixelShader, ID3D11VertexShader};

use super::{
    scopes::{ScopeFrame, ScopeUnk3, ScopeUnk8, ScopeView},
    shader, DeviceContextSwapchain,
};

#[derive(Default)]
pub struct EnabledShaderOverrides {
    pub entity_vs: bool,
    pub entity_ps: bool,
}

pub struct ShaderOverrides {
    pub entity_vs: ID3D11VertexShader,
    pub entity_ps: ID3D11PixelShader,
}

impl ShaderOverrides {
    pub fn load(dcs: &DeviceContextSwapchain) -> anyhow::Result<Self> {
        let vshader_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/overrides/entity.vsh"),
            "main",
            "vs_5_0",
        )
        .unwrap();

        let (entity_vs, _) = shader::load_vshader(dcs, &vshader_blob)?;

        let pshader_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/overrides/entity.psh"),
            "main",
            "ps_5_0",
        )
        .unwrap();

        let (entity_ps, _) = shader::load_pshader(dcs, &pshader_blob)?;

        Ok(Self {
            entity_vs,
            entity_ps,
        })
    }
}

#[derive(Default)]
pub struct ScopeOverrides {
    pub view: ScopeView,
    pub frame: ScopeFrame,
    pub unk3: ScopeUnk3,
    pub unk8: ScopeUnk8,
}
