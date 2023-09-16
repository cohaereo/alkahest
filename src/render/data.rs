use std::sync::Arc;
use std::time::Duration;

use crossbeam::channel::Sender;
use destiny_pkg::{TagHash, TagHash64};
use nohash_hasher::IntMap;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use windows::Win32::Graphics::Direct3D11::*;

use crate::dxgi::DxgiFormat;
use crate::map::ExtendedHash;
use crate::material::Material;
use crate::packages::package_manager;
use crate::render::vertex_layout::InputElement;
use crate::texture::Texture;
use crate::util::LockTracker;

use super::drawcall::ShadingTechnique;
use super::renderer::Renderer;
use super::shader::{load_pshader, load_vshader};
use super::vertex_layout::OutputElement;
use super::{resource_mt, DeviceContextSwapchain};

#[derive(Default)]
pub struct RenderData {
    pub materials: IntMap<TagHash, Material>,
    pub vshaders: IntMap<TagHash, (ID3D11VertexShader, Vec<InputElement>, Vec<u8>)>,
    pub pshaders: IntMap<TagHash, (ID3D11PixelShader, Vec<OutputElement>)>,
    pub textures: IntMap<u64, Texture>,
    pub samplers: IntMap<u64, ID3D11SamplerState>,

    pub vertex_buffers: IntMap<TagHash, (ID3D11Buffer, u32)>,
    pub index_buffers: IntMap<TagHash, (ID3D11Buffer, DxgiFormat)>,
    pub input_layouts: IntMap<u64, ID3D11InputLayout>,
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
    tx_textures: Sender<ExtendedHash>,
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

    #[track_caller]
    pub fn data(&self) -> LockTracker<RwLockReadGuard<RenderData>> {
        #[cfg(feature = "debug_lock")]
        debug!(
            "Thread {:?} acquiring RenderData (read) ({})",
            std::thread::current().id(),
            crate::util::caller_frame!(),
        );

        let l = LockTracker::wrap(
            self.render_data
                .try_read_for(Duration::from_secs(5))
                .expect("Lock timeout"),
        );

        #[cfg(feature = "debug_lock")]
        debug!(
            "Thread {:?} acquired lock #{} (read) ({})",
            std::thread::current().id(),
            l.id(),
            crate::util::caller_frame!(),
        );

        l
    }

    #[track_caller]
    pub fn data_mut(&self) -> LockTracker<RwLockWriteGuard<RenderData>> {
        #[cfg(feature = "debug_lock")]
        debug!(
            "Thread {:?} acquiring RenderData (write) ({})",
            std::thread::current().id(),
            crate::util::caller_frame!(),
        );

        let l = LockTracker::wrap(
            self.render_data
                .try_write_for(Duration::from_secs(5))
                .expect("Lock timeout"),
        );

        #[cfg(feature = "debug_lock")]
        debug!(
            "Thread {:?} acquired lock #{} (write) ({})",
            std::thread::current().id(),
            l.id(),
            crate::util::caller_frame!(),
        );

        l
    }

    /// Load a Texture2D, Texture2D or TextureCube from a hash
    pub fn load_texture(&self, texture: ExtendedHash) {
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

    // pub fn load_sampler(&self, dcs: &DeviceContextSwapchain, hash: TagHash) {
    //     if !hash.is_valid() {
    //         return;
    //     }

    //     let sampler_header_ref = package_manager().get_entry(hash).unwrap().reference;
    //     let sampler_data = package_manager().read_tag(sampler_header_ref).unwrap();

    //     let sampler = unsafe {
    //         dcs.device
    //             .CreateSamplerState(sampler_data.as_ptr() as _)
    //             .expect("Failed to create sampler state")
    //     };

    //     self.data_mut().samplers.insert(hash, sampler);
    // }

    pub fn load_vshader(
        &self,
        dcs: &DeviceContextSwapchain,
        hash: TagHash,
    ) -> Option<(ID3D11VertexShader, Vec<InputElement>, Vec<u8>)> {
        if !hash.is_valid() {
            return None;
        }

        Some(self.data_mut().vshaders.entry(hash).or_insert_with(|| {
            let shader_header_ref = package_manager().get_entry(hash).unwrap().reference;
            let shader_data = package_manager().read_tag(shader_header_ref).unwrap();
            let v = load_vshader(dcs, &shader_data).unwrap();
            (v.0, v.1, shader_data)
        }))
        .cloned()
    }

    pub fn load_pshader(&self, dcs: &DeviceContextSwapchain, hash: TagHash) {
        if !hash.is_valid() {
            return;
        }

        self.data_mut().pshaders.entry(hash).or_insert_with(|| {
            let shader_header_ref = package_manager().get_entry(hash).unwrap().reference;
            let shader_data = package_manager().read_tag(shader_header_ref).unwrap();
            load_pshader(dcs, &shader_data).unwrap()
        });
    }

    pub fn load_material(&self, renderer: &Renderer, material: TagHash) {
        if !material.is_valid() {
            return;
        }

        self.data_mut()
            .materials
            .entry(material)
            .or_insert_with(|| {
                Material::load(
                    renderer,
                    package_manager().read_tag_struct(material).unwrap(),
                    material,
                    false,
                )
            });
    }
}
