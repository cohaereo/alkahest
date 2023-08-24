use destiny_pkg::TagHash;
use nohash_hasher::IntMap;
use windows::Win32::Graphics::Direct3D11::*;

use crate::material::Material;
use crate::texture::Texture;
use crate::types::Vector4;
use crate::vertex_layout::InputElement;

use super::drawcall::ShadingTechnique;
use super::ConstantBuffer;

pub struct RenderData {
    pub materials: IntMap<u32, Material>,
    pub vshaders: IntMap<u32, (ID3D11VertexShader, Option<ID3D11InputLayout>)>,
    pub pshaders: IntMap<u32, (ID3D11PixelShader, Vec<InputElement>)>,
    pub cbuffers_vs: IntMap<u32, ConstantBuffer<Vector4>>,
    pub cbuffers_ps: IntMap<u32, ConstantBuffer<Vector4>>,
    pub textures: IntMap<u32, Texture>,
    pub samplers: IntMap<u32, ID3D11SamplerState>,
}

impl RenderData {
    // Get the shading technique for a material based on it's pixel shader output signature
    pub fn material_shading_technique(&self, material: TagHash) -> Option<ShadingTechnique> {
        let pixel_shader = self.materials.get(&material.0)?.0.pixel_shader;

        if self.pshaders.get(&pixel_shader.0)?.1.len() == 1 {
            Some(ShadingTechnique::Forward)
        } else {
            Some(ShadingTechnique::Deferred)
        }
    }
}
