use std::sync::Arc;

use crate::util::image::Png;
use crate::util::RwLock;
use crossbeam::channel::Sender;
use destiny_pkg::TagHash;
use nohash_hasher::IntMap;
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};
use windows::Win32::Graphics::Direct3D11::*;

use crate::dxgi::DxgiFormat;
use crate::material::Technique;
use crate::packages::package_manager;
use crate::render::vertex_layout::InputElement;
use crate::structure::ExtendedHash;
use crate::texture::Texture;

use super::drawcall::ShadingMode;
use super::renderer::Renderer;
use super::shader::{load_pshader, load_vshader};
use super::vertex_layout::OutputElement;
use super::{resource_mt, DeviceContextSwapchain};

pub struct RenderData {
    pub materials: IntMap<TagHash, Technique>,
    pub vshaders: IntMap<TagHash, (ID3D11VertexShader, Vec<InputElement>, Vec<u8>)>,
    pub pshaders: IntMap<TagHash, (ID3D11PixelShader, Vec<OutputElement>)>,
    pub textures: IntMap<u64, Texture>,
    pub samplers: IntMap<u64, ID3D11SamplerState>,

    pub vertex_buffers: IntMap<TagHash, (ID3D11Buffer, u32, Option<ID3D11ShaderResourceView>)>,
    pub index_buffers: IntMap<TagHash, (ID3D11Buffer, DxgiFormat)>,
    pub input_layouts: IntMap<u64, ID3D11InputLayout>,

    pub fallback_texture: Texture,
    /// All the colors you need
    pub rainbow_texture: Texture,
    pub debug_textures: Vec<Texture>,

    pub matcap: Texture,
    // A 2x2 white texture
    pub white: Texture,
    pub blend_texture: Texture,
    pub blend_texture15: Texture,

    pub solid_texture_red: Texture,
    pub solid_texture_green: Texture,
    pub solid_texture_blue: Texture,
    pub solid_texture_magenta: Texture,
}

impl RenderData {
    pub fn new(dcs: &DeviceContextSwapchain) -> anyhow::Result<Self> {
        let fallback_texture = Texture::load_png(
            dcs,
            &Png::from_bytes(include_bytes!("../../assets/textures/fallback.png"))?,
            Some("red/black checkerboard"),
        )?;

        let rainbow_texture = Texture::load_png(
            dcs,
            &Png::from_bytes(include_bytes!("../../assets/textures/rainbow.png"))?,
            Some("raaaainbow"),
        )?;

        const DEBUG_TEXTURE_DATA: [&[u8]; 8] = [
            include_bytes!("../../assets/textures/debug0.png"),
            include_bytes!("../../assets/textures/debug1.png"),
            include_bytes!("../../assets/textures/debug2.png"),
            include_bytes!("../../assets/textures/debug3.png"),
            include_bytes!("../../assets/textures/debug4.png"),
            include_bytes!("../../assets/textures/debug5.png"),
            include_bytes!("../../assets/textures/debug6.png"),
            include_bytes!("../../assets/textures/debug7.png"),
        ];

        let mut debug_textures = vec![];
        for (i, d) in DEBUG_TEXTURE_DATA.iter().enumerate() {
            debug_textures.push(Texture::load_png(
                dcs,
                &Png::from_bytes(d)?,
                Some(&format!("debug texture #{i}")),
            )?);
        }

        const MATCAP_DATA: &[u8] = include_bytes!("../../assets/textures/matcap.png");
        let matcap = Texture::load_png(
            dcs,
            &Png::from_bytes(MATCAP_DATA)?,
            Some("Basic shading matcap"),
        )?;

        let white = Texture::load_2d_raw(
            dcs,
            1,
            1,
            &[0xffu8; 4],
            DxgiFormat::R8G8B8A8_UNORM,
            Some("1x1 white"),
        )?;

        let blend_texture = Texture::load_3d_raw(
            dcs,
            2,
            2,
            2,
            &[0x50, 0x50, 0x50, 0xff].repeat(2 * 2 * 2),
            DxgiFormat::R8G8B8A8_UNORM,
            Some("2x2x2 blend factor"),
        )?;

        let blend_texture15 = Texture::load_2d_raw(
            dcs,
            2,
            2,
            &[0x50, 0x50, 0x50, 0x50].repeat(2 * 2),
            DxgiFormat::R8G8B8A8_UNORM,
            Some("2x2 blend factor15"),
        )?;

        let solid_texture_red = Texture::load_2d_raw(
            dcs,
            2,
            2,
            &[0xff, 0x00, 0x00, 0xff].repeat(2 * 2),
            DxgiFormat::R8G8B8A8_UNORM,
            Some("2x2 solid red"),
        )?;

        let solid_texture_green = Texture::load_2d_raw(
            dcs,
            2,
            2,
            &[0x00, 0xff, 0x00, 0xff].repeat(2 * 2),
            DxgiFormat::R8G8B8A8_UNORM,
            Some("2x2 solid green"),
        )?;

        let solid_texture_blue = Texture::load_2d_raw(
            dcs,
            2,
            2,
            &[0x00, 0x00, 0xff, 0xff].repeat(2 * 2),
            DxgiFormat::R8G8B8A8_UNORM,
            Some("2x2 solid blue"),
        )?;

        let solid_texture_magenta = Texture::load_2d_raw(
            dcs,
            2,
            2,
            &[0xff, 0x00, 0xff, 0xff].repeat(2 * 2),
            DxgiFormat::R8G8B8A8_UNORM,
            Some("2x2 solid magenta"),
        )?;

        Ok(RenderData {
            materials: Default::default(),
            vshaders: Default::default(),
            pshaders: Default::default(),
            textures: Default::default(),
            samplers: Default::default(),
            vertex_buffers: Default::default(),
            index_buffers: Default::default(),
            input_layouts: Default::default(),
            fallback_texture,
            rainbow_texture,
            debug_textures,
            matcap,
            white,
            blend_texture,
            blend_texture15,
            solid_texture_red,
            solid_texture_green,
            solid_texture_blue,
            solid_texture_magenta,
        })
    }

    // Get the shading technique for a material based on it's pixel shader output signature
    pub fn material_shading_technique(&self, material: TagHash) -> Option<ShadingMode> {
        let pixel_shader = self.materials.get(&material)?.shader_pixel.shader;

        if self.pshaders.get(&pixel_shader)?.1.len() == 1 {
            Some(ShadingMode::Forward)
        } else {
            Some(ShadingMode::Deferred)
        }
    }
}

pub struct RenderDataManager {
    tx_textures: Sender<ExtendedHash>,
    tx_buffers: Sender<(TagHash, bool)>,
    // tx_shaders: Sender<TagHash>,
    render_data: Arc<RwLock<RenderData>>,
}

#[cfg(feature = "debug_lock")]
use crate::util::LockTracker;

impl RenderDataManager {
    pub fn new(dcs: Arc<DeviceContextSwapchain>) -> Self {
        let render_data = Arc::new(RwLock::new(RenderData::new(&dcs).unwrap()));
        let tx_textures = resource_mt::thread_textures(dcs.clone(), render_data.clone());
        let tx_buffers = resource_mt::thread_buffers(dcs.clone(), render_data.clone());

        Self {
            tx_textures,
            tx_buffers,
            render_data,
        }
    }

    #[cfg(feature = "debug_lock")]
    pub fn data(&self) -> LockTracker<RwLockReadGuard<'_, RenderData>> {
        self.render_data.read()
    }

    #[cfg(feature = "debug_lock")]
    pub fn data_mut(&self) -> LockTracker<RwLockWriteGuard<'_, RenderData>> {
        self.render_data.write()
    }

    #[cfg(not(feature = "debug_lock"))]
    pub fn data(&self) -> RwLockReadGuard<'_, RenderData> {
        self.render_data.read()
    }

    #[cfg(not(feature = "debug_lock"))]
    pub fn data_mut(&self) -> RwLockWriteGuard<'_, RenderData> {
        self.render_data.write()
    }

    /// Load a Texture2D, Texture2D or TextureCube from a hash
    pub fn load_texture(&self, texture: ExtendedHash) {
        self.tx_textures
            .send(texture)
            .expect("Failed to send load texture request");
    }

    /// Load a vertex or index buffer from a hash
    pub fn load_buffer(&self, buffer: TagHash, create_srv: bool) {
        self.tx_buffers
            .send((buffer, create_srv))
            .expect("Failed to send load buffer request");
    }

    // pub fn load_sampler(&self, dcs: &DeviceContextSwapchain, hash: TagHash) {
    //     if !hash.is_some() {
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
        if !hash.is_some() {
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
        if !hash.is_some() {
            return;
        }

        self.data_mut().pshaders.entry(hash).or_insert_with(|| {
            let shader_header_ref = package_manager().get_entry(hash).unwrap().reference;
            let shader_data = package_manager().read_tag(shader_header_ref).unwrap();
            load_pshader(dcs, &shader_data).unwrap()
        });
    }

    pub fn load_material(&self, renderer: &Renderer, material: TagHash) {
        if !material.is_some() {
            return;
        }

        self.data_mut()
            .materials
            .entry(material)
            .or_insert_with(|| {
                Technique::load(
                    renderer,
                    package_manager().read_tag_struct(material).unwrap(),
                    material,
                    false,
                )
            });
    }
}
