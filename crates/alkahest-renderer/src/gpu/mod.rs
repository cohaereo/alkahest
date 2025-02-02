pub mod adapter;
pub mod buffer;
mod d3dstate;
pub mod debug;
pub mod global_state;
pub mod texture;
pub mod util;

use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering},
        Arc,
    },
};

use adapter::GpuAdapter;
use alkahest_data::{
    dxgi::DxgiFormat, geometry::EPrimitiveType, technique::StateSelection, tfx::TfxShaderStage,
};
use crossbeam::atomic::AtomicCell;
use debug::PendingGpuTimestampRange;
use parking_lot::{Mutex, ReentrantMutexGuard};
use windows::Win32::Graphics::{Direct3D::*, Direct3D11::*};

use crate::{
    gpu::{global_state::RenderStates, texture::Texture, util::UtilResources},
    loaders::vertex_buffer::VertexBuffer,
    util::image::Png,
};

pub struct GpuContext {
    pub adapter: Arc<GpuAdapter>,
    pub fallback_texture: Texture,
    pub color0_fallback: VertexBuffer,
    pub color_ao_fallback: VertexBuffer,

    pub atmos_depth_angle_lookup_tmp: Texture,
    pub sky_hemisphere_placeholder: Texture,
    pub shadowmap_vs_t2: Texture,
    pub white_texture: Texture,
    pub light_grey_texture: Texture,
    pub grey_texture: Texture,
    pub dark_grey_texture: Texture,
    pub black_texture: Texture,

    pub states: RenderStates,

    current_blend_state: AtomicUsize,
    current_input_layout: AtomicUsize,
    current_rasterizer_state: AtomicUsize,
    current_depth_bias: AtomicUsize,
    current_input_topology: AtomicI32,
    current_depth_state: AtomicUsize,
    use_flipped_depth_comparison: AtomicBool,

    pub current_states: AtomicCell<StateSelection>,

    pub util_resources: UtilResources,
    pub custom_pixel_shader: Option<ID3D11PixelShader>,

    pending_timestamp_queries: Mutex<Vec<PendingGpuTimestampRange>>,
}

impl GpuContext {
    pub fn create(adapter: &Arc<GpuAdapter>) -> anyhow::Result<Arc<Self>> {
        let states = RenderStates::new(&adapter.device)?;

        let fallback_texture = Texture::load_png(
            &adapter.device,
            &Png::from_bytes(include_bytes!("../../assets/textures/fallback.png"))?,
            Some("Fallback Texture"),
        )?;

        let white_texture = Texture::load_2d_raw(
            &adapter.device,
            1,
            1,
            &[255, 255, 255, 255],
            DxgiFormat::R8G8B8A8_UNORM_SRGB,
            Some("White Texture"),
        )?;

        let light_grey_texture = Texture::load_2d_raw(
            &adapter.device,
            1,
            1,
            &[196, 196, 196, 196],
            DxgiFormat::R8G8B8A8_UNORM_SRGB,
            Some("Grey Texture"),
        )?;

        let grey_texture = Texture::load_2d_raw(
            &adapter.device,
            1,
            1,
            &[127, 127, 127, 127],
            DxgiFormat::R8G8B8A8_UNORM_SRGB,
            Some("Grey Texture"),
        )?;

        let dark_grey_texture = Texture::load_2d_raw(
            &adapter.device,
            1,
            1,
            &[40, 40, 40, 127],
            DxgiFormat::R8G8B8A8_UNORM_SRGB,
            Some("Grey Texture"),
        )?;

        let black_texture = Texture::load_2d_raw(
            &adapter.device,
            1,
            1,
            &[0, 0, 0, 0],
            DxgiFormat::R8G8B8A8_UNORM_SRGB,
            Some("Black Texture"),
        )?;

        let shadowmap_vs_t2 = Texture::load_2d_raw(
            &adapter.device,
            1,
            1,
            &[0, 0, 255, 255],
            DxgiFormat::R8G8B8A8_UNORM_SRGB,
            Some("shadowmap_vs_t2"),
        )?;

        let color0_fallback = VertexBuffer::load_data(&adapter.device, &[0, 0, 255, 255], 4)?
            .with_name("color0_fallback");
        let color_ao_fallback = VertexBuffer::load_data(&adapter.device, &[255; 128 * 4], 4)?
            .with_name("color_ao_fallback");

        let sky_hemisphere_placeholder = Texture::load_png(
            &adapter.device,
            &Png::from_bytes(include_bytes!(
                "../../assets/textures/sky_hemisphere_placeholder.png"
            ))?,
            Some("sky_hemisphere_placeholder.png"),
        )?;

        let atmos_depth_angle_lookup_tmp = Texture::load_png(
            &adapter.device,
            &Png::from_bytes(include_bytes!(
                "../../assets/textures/depth_angle_lookup_temp.png"
            ))?,
            Some("depth_angle_lookup_temp.png"),
        )?;

        Ok(Arc::new(Self {
            util_resources: UtilResources::new(&adapter.device),

            adapter: adapter.clone(),
            fallback_texture,
            color0_fallback,
            color_ao_fallback,
            sky_hemisphere_placeholder,
            atmos_depth_angle_lookup_tmp,
            white_texture,
            light_grey_texture,
            grey_texture,
            dark_grey_texture,
            black_texture,
            shadowmap_vs_t2,

            states,

            current_blend_state: AtomicUsize::new(usize::MAX),
            current_input_layout: AtomicUsize::new(usize::MAX),
            current_rasterizer_state: AtomicUsize::new(usize::MAX),
            current_depth_bias: AtomicUsize::new(usize::MAX),
            current_input_topology: AtomicI32::new(-1),
            current_depth_state: AtomicUsize::new(usize::MAX),
            use_flipped_depth_comparison: AtomicBool::new(false),

            current_states: AtomicCell::new(StateSelection::new(
                Some(0),
                Some(0),
                Some(2),
                Some(0),
            )),
            custom_pixel_shader: None,

            pending_timestamp_queries: Mutex::new(Vec::new()),
        }))
    }

    /// The device context may only be accessed from one thread at a time, so calling this method will lock the context for the current thread.
    #[inline(always)]
    pub fn lock_context(&self) -> ReentrantMutexGuard<ID3D11DeviceContext> {
        self.context.lock()
    }
}

impl Deref for GpuContext {
    type Target = GpuAdapter;

    fn deref(&self) -> &Self::Target {
        &self.adapter
    }
}

impl GpuContext {
    pub fn begin_frame(&self) {
        self.pending_timestamp_queries.lock().clear();
        // for pending_timestamp in std::mem::take(&mut *self.pending_timestamp_queries.lock()) {
        //     let timestamp = pending_timestamp.resolve_blocking(self);
        //     if !timestamp.disjoint {
        //         println!(
        //             "Resolved timestamp {} - {}ms",
        //             &timestamp.label,
        //             timestamp.to_miliseconds_f32()
        //         );
        //     }
        // }
        // println!();

        unsafe {
            // TODO(cohae): Clearing the state causes maps like bannerfall to use a point fill mode (which doesn't exist in dx11????)
            // self.context.ClearState();

            self.lock_context().RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: self.swapchain_resolution.load().0 as f32,
                Height: self.swapchain_resolution.load().1 as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]));
        }

        self.reset_states();
    }

    fn reset_states(&self) {
        // Reset current states
        self.current_blend_state
            .store(usize::MAX, Ordering::Relaxed);
        self.current_depth_state
            .store(usize::MAX, Ordering::Relaxed);
        self.current_input_layout
            .store(usize::MAX, Ordering::Relaxed);
        self.current_rasterizer_state
            .store(usize::MAX, Ordering::Relaxed);
        self.current_depth_bias.store(usize::MAX, Ordering::Relaxed);
    }

    pub fn flush_states(&self) {
        self.reset_states();
        let states = self.current_states.load();
        if let Some(blend) = states.blend_state() {
            self.set_blend_state(blend);
        }
        if let Some(depth_stencil) = states.depth_stencil_state() {
            self.set_depth_stencil_state(depth_stencil);
        }
        if let Some(rasterizer) = states.rasterizer_state() {
            self.set_rasterizer_state(rasterizer);
        }
        if let Some(depth_bias) = states.depth_bias_state() {
            self.set_depth_bias(depth_bias);
        }
    }

    pub fn set_depth_mode(&self, mode: DepthMode) {
        let flipped = mode == DepthMode::Flipped;
        self.use_flipped_depth_comparison
            .store(flipped, Ordering::Relaxed);
        // cohae: Since the depth/stencil state only checks the index and not whether we changed the flipped state, we need to flush the depth state manually
        self.flush_states();
    }
}

impl GpuContext {
    pub fn set_blend_state(&self, index: usize) {
        if self.current_blend_state.load(Ordering::Relaxed) != index {
            unsafe {
                self.lock_context().OMSetBlendState(
                    &self.states.blend_states[index],
                    Some(&[1.0, 1.0, 1.0, 1.0]),
                    0xFFFFFFFF,
                );
            }
            self.current_blend_state.store(index, Ordering::Relaxed);
        }
    }

    pub fn set_depth_stencil_state(&self, index: usize) {
        if self.current_depth_state.load(Ordering::Relaxed) != index {
            let states = &self.states.depth_stencil_states[index];
            unsafe {
                self.lock_context().OMSetDepthStencilState(
                    if self.use_flipped_depth_comparison.load(Ordering::Relaxed) {
                        &states.1
                    } else {
                        &states.0
                    },
                    0,
                );
            }
            self.current_depth_state.store(index, Ordering::Relaxed);
        }
    }

    pub fn set_rasterizer_state(&self, index: usize) {
        if self.current_rasterizer_state.load(Ordering::Relaxed) != index {
            unsafe {
                let depth_bias = self.current_depth_bias.load(Ordering::Relaxed);
                if index < 9 && depth_bias < 9 {
                    self.lock_context()
                        .RSSetState(self.states.rasterizer_states[depth_bias][index].as_ref());
                }
            }
            self.current_rasterizer_state
                .store(index, Ordering::Relaxed);
        }
    }

    pub fn set_depth_bias(&self, index: usize) {
        if self.current_depth_bias.load(Ordering::Relaxed) != index {
            unsafe {
                let rasterizer_state = self.current_rasterizer_state.load(Ordering::Relaxed);
                if index < 9 && rasterizer_state < 9 {
                    self.lock_context().RSSetState(
                        self.states.rasterizer_states[index][rasterizer_state].as_ref(),
                    );
                }
            }
            self.current_depth_bias.store(index, Ordering::Relaxed);
        }
    }

    pub fn set_input_layout(&self, index: usize) {
        if self.current_input_layout.load(Ordering::Relaxed) != index {
            unsafe {
                self.lock_context()
                    .IASetInputLayout(&self.states.input_layouts[index]);
            }
            self.current_input_layout.store(index, Ordering::Relaxed);
        }
    }

    pub fn set_input_topology(&self, topology: EPrimitiveType) {
        if self.current_input_topology.load(Ordering::Relaxed) != topology as i32 {
            unsafe {
                self.lock_context().IASetPrimitiveTopology(match topology {
                    EPrimitiveType::PointList => D3D11_PRIMITIVE_TOPOLOGY_POINTLIST,
                    EPrimitiveType::LineList => D3D11_PRIMITIVE_TOPOLOGY_LINELIST,
                    EPrimitiveType::LineStrip => D3D11_PRIMITIVE_TOPOLOGY_LINESTRIP,
                    EPrimitiveType::Triangles => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
                    EPrimitiveType::TriangleStrip => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
                });
            }
            self.current_input_topology
                .store(topology as i32, Ordering::Relaxed);
        }
    }

    pub fn bind_srv(
        &self,
        view: Option<ID3D11ShaderResourceView>,
        slot: u32,
        stage: TfxShaderStage,
    ) {
        let ctx = self.lock_context();
        unsafe {
            match stage {
                TfxShaderStage::Vertex => ctx.VSSetShaderResources(slot, Some(&[view])),
                TfxShaderStage::Hull => ctx.HSSetShaderResources(slot, Some(&[view])),
                TfxShaderStage::Domain => ctx.DSSetShaderResources(slot, Some(&[view])),
                TfxShaderStage::Geometry => ctx.GSSetShaderResources(slot, Some(&[view])),
                TfxShaderStage::Pixel => ctx.PSSetShaderResources(slot, Some(&[view])),
                TfxShaderStage::Compute => ctx.CSSetShaderResources(slot, Some(&[view])),
            }
        }
    }
}

impl GpuContext {
    pub fn bind_cbuffer(&self, slot: u32, buffer: Option<ID3D11Buffer>, stage: TfxShaderStage) {
        unsafe {
            let ctx = self.lock_context();
            match stage {
                TfxShaderStage::Vertex => ctx.VSSetConstantBuffers(slot, Some(&[buffer])),
                TfxShaderStage::Pixel => ctx.PSSetConstantBuffers(slot, Some(&[buffer])),
                TfxShaderStage::Geometry => ctx.GSSetConstantBuffers(slot, Some(&[buffer])),
                TfxShaderStage::Compute => ctx.CSSetConstantBuffers(slot, Some(&[buffer])),
                TfxShaderStage::Hull => ctx.HSSetConstantBuffers(slot, Some(&[buffer])),
                TfxShaderStage::Domain => ctx.DSSetConstantBuffers(slot, Some(&[buffer])),
            }
        }
    }

    pub fn bind_pixel_shader<'a, S: Into<Option<&'a ID3D11PixelShader>>>(&self, shader: S) {
        let shader = shader.into();
        if shader.is_some() {
            unsafe {
                self.lock_context()
                    .PSSetShader(self.custom_pixel_shader.as_ref().or(shader), None);
            }
        } else {
            unsafe {
                self.lock_context().PSSetShader(None, None);
            }
        }
    }
}

unsafe impl Send for GpuContext {}
unsafe impl Sync for GpuContext {}

#[derive(PartialEq)]
pub enum DepthMode {
    Normal,
    /// Used for rendering shadowmaps
    Flipped,
}
