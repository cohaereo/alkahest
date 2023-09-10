use std::sync::Arc;

use crossbeam::channel::Sender;
use destiny_pkg::TagHash;
use nohash_hasher::IntMap;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use windows::Win32::Graphics::Direct3D11::*;

use crate::material::Material;
use crate::texture::Texture;
use crate::types::Vector4;
use crate::vertex_layout::InputElement;

use super::drawcall::ShadingTechnique;
use super::{resource_mt, ConstantBuffer, DeviceContextSwapchain};

#[derive(Default)]
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
        let pixel_shader = self.materials.get(&material.0)?.pixel_shader;

        if self.pshaders.get(&pixel_shader.0)?.1.len() == 1 {
            Some(ShadingTechnique::Forward)
        } else {
            Some(ShadingTechnique::Deferred)
        }
    }
}

pub struct RenderDataManager {
    tx_textures: Sender<TagHash>,
    // tx_buffers: Sender<TagHash>,
    // tx_shaders: Sender<TagHash>,
    render_data: Arc<RwLock<RenderData>>,
}

impl RenderDataManager {
    pub fn new(dcs: Arc<DeviceContextSwapchain>) -> Self {
        let render_data = Arc::new(RwLock::new(RenderData::default()));
        let tx_textures = resource_mt::thread_textures(dcs.clone(), render_data.clone());

        Self {
            tx_textures,
            render_data,
        }
    }

    pub fn data(&self) -> RwLockReadGuard<RenderData> {
        self.render_data.read()
    }

    pub fn data_mut(&self) -> RwLockWriteGuard<RenderData> {
        self.render_data.write()
    }

    pub fn load_texture(&self, texture: TagHash) {
        self.tx_textures
            .send(texture)
            .expect("Failed to send load texture request");
    }
}
