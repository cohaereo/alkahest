use windows::Win32::Graphics::Direct3D11::{ID3D11PixelShader, ID3D11VertexShader};

use super::{
    scopes::{ScopeFrame, ScopeUnk2, ScopeUnk3, ScopeUnk8, ScopeView},
    shader, DeviceContextSwapchain,
};

pub struct EnabledShaderOverrides {
    pub entity_vs: bool,
    pub entity_ps: bool,
}

impl Default for EnabledShaderOverrides {
    fn default() -> Self {
        Self {
            // TODO(cohae): remove when we fix entity VS
            entity_vs: true,
            entity_ps: false,
        }
    }
}

pub struct ShaderOverrides {
    pub entity_vs: ID3D11VertexShader,
    pub entity_ps_deferred: ID3D11PixelShader,
    pub entity_ps_forward: ID3D11PixelShader,
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
            include_str!("../../assets/shaders/overrides/entity_deferred.psh"),
            "main",
            "ps_5_0",
        )
        .unwrap();

        let (entity_ps_deferred, _) = shader::load_pshader(dcs, &pshader_blob)?;

        let pshader_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/overrides/entity_forward.psh"),
            "main",
            "ps_5_0",
        )
        .unwrap();

        let (entity_ps_forward, _) = shader::load_pshader(dcs, &pshader_blob)?;

        Ok(Self {
            entity_vs,
            entity_ps_deferred,
            entity_ps_forward,
        })
    }
}

#[derive(Default)]
pub struct ScopeOverrides {
    pub view: ScopeView,
    pub frame: ScopeFrame,
    pub unk2: ScopeUnk2,
    pub unk3: ScopeUnk3,
    pub unk8: ScopeUnk8,
}
