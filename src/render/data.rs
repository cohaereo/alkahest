use nohash_hasher::IntMap;
use windows::Win32::Graphics::Direct3D11::*;

use crate::material;
use crate::texture::LoadedTexture;
use crate::types::Vector4;

use super::ConstantBuffer;

pub struct RenderData {
    pub materials: IntMap<u32, material::Unk808071e8>,
    pub vshaders: IntMap<u32, (ID3D11VertexShader, Option<ID3D11InputLayout>)>,
    pub pshaders: IntMap<u32, ID3D11PixelShader>,
    pub cbuffers_vs: IntMap<u32, ConstantBuffer<Vector4>>,
    pub cbuffers_ps: IntMap<u32, ConstantBuffer<Vector4>>,
    pub textures: IntMap<u32, LoadedTexture>,
    pub samplers: IntMap<u32, ID3D11SamplerState>,
}
