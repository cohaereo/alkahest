use windows::Win32::Graphics::Direct3D11::{
    ID3D11PixelShader, ID3D11SamplerState, ID3D11VertexShader,
};

use super::{
    scopes::{ScopeFrame, ScopeTransparent, ScopeTransparentAdvanced, ScopeUnk3, ScopeView},
    shader, DeviceContextSwapchain,
};

pub struct EnabledShaderOverrides {
    pub entity_vs: bool,
    pub entity_ps: bool,

    pub terrain_ps: bool,
}

impl Default for EnabledShaderOverrides {
    fn default() -> Self {
        Self {
            // TODO(cohae): disable when we fix entity VS
            entity_vs: true,
            entity_ps: false,
            terrain_ps: false,
        }
    }
}

pub struct ShaderOverrides {
    pub entity_vs: ID3D11VertexShader,
    pub entity_ps_deferred: ID3D11PixelShader,
    pub entity_ps_forward: ID3D11PixelShader,

    pub terrain_ps: ID3D11PixelShader,
    pub terrain_debug_sampler: ID3D11SamplerState,
}

impl ShaderOverrides {
    pub fn load(dcs: &DeviceContextSwapchain) -> anyhow::Result<Self> {
        let vshader_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/overrides/entity.vsh"),
            "main",
            "vs_5_0",
            "entity.vsh",
        )
        .unwrap();

        let (entity_vs, _) = shader::load_vshader(dcs, &vshader_blob)?;

        let pshader_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/overrides/entity_deferred.psh"),
            "main",
            "ps_5_0",
            "entity_deferred.psh",
        )
        .unwrap();

        let (entity_ps_deferred, _) = shader::load_pshader(dcs, &pshader_blob)?;

        let pshader_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/overrides/entity_forward.psh"),
            "main",
            "ps_5_0",
            "entity_forward.psh",
        )
        .unwrap();

        let (entity_ps_forward, _) = shader::load_pshader(dcs, &pshader_blob)?;

        let pshader_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/overrides/terrain.psh"),
            "main",
            "ps_5_0",
            "terrain.psh",
        )
        .unwrap();

        let (terrain_ps, _) = shader::load_pshader(dcs, &pshader_blob)?;

        let terrain_debug_sampler = unsafe {
            use windows::Win32::Graphics::Direct3D11::*;
            let desc = D3D11_SAMPLER_DESC {
                Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                AddressU: D3D11_TEXTURE_ADDRESS_WRAP,
                AddressV: D3D11_TEXTURE_ADDRESS_WRAP,
                AddressW: D3D11_TEXTURE_ADDRESS_WRAP,
                MipLODBias: 0.,
                ComparisonFunc: D3D11_COMPARISON_ALWAYS,
                MinLOD: 0.,
                MaxLOD: 0.,
                BorderColor: [1., 0., 0., 1.],
                ..Default::default()
            };

            dcs.device.CreateSamplerState(&desc)?
        };

        Ok(Self {
            entity_vs,
            entity_ps_deferred,
            entity_ps_forward,
            terrain_ps,
            terrain_debug_sampler,
        })
    }
}

#[derive(Default)]
pub struct ScopeOverrides {
    pub view: ScopeView,
    pub frame: ScopeFrame,
    pub transparent: ScopeTransparent,
    pub unk3: ScopeUnk3,
    pub transparent_advanced: ScopeTransparentAdvanced,
}
