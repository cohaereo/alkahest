use std::sync::Arc;

use crossbeam::channel::Sender;
use destiny_pkg::TagHash;
use nohash_hasher::IntMap;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use windows::Win32::Graphics::Direct3D11::*;

use crate::dxgi::DxgiFormat;
use crate::material::Material;
use crate::texture::Texture;
use crate::vertex_layout::InputElement;

use super::drawcall::ShadingTechnique;
use super::{resource_mt, DeviceContextSwapchain};

#[derive(Default)]
pub struct RenderData {
    pub materials: IntMap<TagHash, Material>,
    pub vshaders: IntMap<TagHash, (ID3D11VertexShader, Option<ID3D11InputLayout>)>,
    pub pshaders: IntMap<TagHash, (ID3D11PixelShader, Vec<InputElement>)>,
    pub textures: IntMap<TagHash, Texture>,
    pub samplers: IntMap<TagHash, ID3D11SamplerState>,

    pub vertex_buffers: IntMap<TagHash, (ID3D11Buffer, u32)>,
    pub index_buffers: IntMap<TagHash, (ID3D11Buffer, DxgiFormat)>,
}

impl RenderData {
    // Get the shading technique for a material based on it's pixel shader output signature
    pub fn material_shading_technique(&self, material: TagHash) -> Option<ShadingTechnique> {
        let pixel_shader = self.materials.get(&material)?.pixel_shader;

        if self.pshaders.get(&pixel_shader)?.1.len() == 1 {
            Some(ShadingTechnique::Forward)
        } else {
            Some(ShadingTechnique::Deferred)
        }
    }
}

pub struct RenderDataManager {
    tx_textures: Sender<TagHash>,
    tx_buffers: Sender<TagHash>,
    // tx_shaders: Sender<TagHash>,
    render_data: Arc<RwLock<RenderData>>,
}

impl RenderDataManager {
    pub fn new(dcs: Arc<DeviceContextSwapchain>) -> Self {
        let render_data = Arc::new(RwLock::new(RenderData::default()));
        let tx_textures = resource_mt::thread_textures(dcs.clone(), render_data.clone());
        let tx_buffers = resource_mt::thread_buffers(dcs.clone(), render_data.clone());

        Self {
            tx_textures,
            tx_buffers,
            render_data,
        }
    }

    pub fn data(&self) -> RwLockReadGuard<RenderData> {
        self.render_data.read()
    }

    pub fn data_mut(&self) -> RwLockWriteGuard<RenderData> {
        self.render_data.write()
    }

    /// Load a Texture2D, Texture2D or TextureCube from a hash
    pub fn load_texture(&self, texture: TagHash) {
        self.tx_textures
            .send(texture)
            .expect("Failed to send load texture request");
    }

    /// Load a vertex or index buffer from a hash
    pub fn load_buffer(&self, buffer: TagHash) {
        self.tx_buffers
            .send(buffer)
            .expect("Failed to send load buffer request");
    }
}
