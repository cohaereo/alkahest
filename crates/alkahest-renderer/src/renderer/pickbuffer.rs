use std::mem::size_of;

use alkahest_data::{
    dxgi::DxgiFormat,
    geometry::EPrimitiveType,
    technique::StateSelection,
    tfx::{TfxRenderStage, TfxShaderStage},
};
use anyhow::Context;
use hecs::Entity;
use windows::Win32::Graphics::Direct3D11::{ID3D11PixelShader, ID3D11VertexShader, D3D11_MAP_READ};

use crate::{
    ecs::Scene,
    gpu::{buffer::ConstantBuffer, util::DxDeviceExt, GpuContext, SharedGpuContext},
    gpu_event, include_dxbc,
    renderer::{
        gbuffer::{CpuStagingBuffer, DepthState, RenderTarget},
        Renderer,
    },
    util::Hocus,
};

impl Renderer {
    pub(super) fn draw_pickbuffer(&self, scene: &Scene, selected: Option<Entity>) {
        gpu_event!(self.gpu, "pickbuffer");
        self.pickbuffer.start(&self.gpu);
        self.pickbuffer.pick_cb.bind(7, TfxShaderStage::Pixel);
        self.gpu.bind_pixel_shader(&self.pickbuffer.pick_ps);
        *self.gpu.custom_pixel_shader.pocus() = Some(self.pickbuffer.pick_ps.clone());
        self.run_renderstage_systems(scene, TfxRenderStage::DepthPrepass);
        *self.gpu.custom_pixel_shader.pocus() = None;
        self.pickbuffer.end();
    }
}

pub struct Pickbuffer {
    pub outline_depth: DepthState,
    pub pick_buffer: RenderTarget,
    pub pick_buffer_staging: CpuStagingBuffer,

    clear_vs: ID3D11VertexShader,
    clear_ps: ID3D11PixelShader,

    pick_ps: ID3D11PixelShader,
    pick_cb: ConstantBuffer<u32>,
    pub(crate) active_entity: Option<Entity>,
}

impl Pickbuffer {
    pub fn new(gctx: SharedGpuContext, window_size: (u32, u32)) -> anyhow::Result<Self> {
        let clear_vs = gctx
            .device
            .load_vertex_shader(include_dxbc!(vs "gui/pickbuffer_clear.hlsl"))?;
        let clear_ps = gctx
            .device
            .load_pixel_shader(include_dxbc!(ps "gui/pickbuffer_clear.hlsl"))?;
        let pick_ps = gctx
            .device
            .load_pixel_shader(include_dxbc!(ps "gui/pickbuffer.hlsl"))?;

        Ok(Self {
            outline_depth: DepthState::create(gctx.clone(), window_size)
                .context("Outline Depth")?,
            pick_buffer: RenderTarget::create(
                window_size,
                DxgiFormat::R32_UINT,
                gctx.clone(),
                "Entity_Pickbuffer",
            )
            .context("Entity_Pickbuffer")?,
            pick_buffer_staging: CpuStagingBuffer::create(
                window_size,
                DxgiFormat::R32_UINT,
                gctx.clone(),
                "Entity_Pickbuffer_Staging",
            )
            .context("Entity_Pickbuffer_Staging")?,

            clear_vs,
            clear_ps,
            pick_ps,
            pick_cb: ConstantBuffer::create(gctx.clone(), None)?,
            active_entity: None,
        })
    }

    pub fn resize(&mut self, mut new_size: (u32, u32)) -> anyhow::Result<()> {
        if new_size.0 == 0 || new_size.1 == 0 {
            new_size = (1, 1);
        }

        self.outline_depth
            .resize(new_size)
            .context("Outline Depth")?;
        self.pick_buffer
            .resize(new_size)
            .context("Entity_Pickbuffer")?;
        self.pick_buffer_staging
            .resize(new_size)
            .context("Entity_Pickbuffer_Staging")?;

        Ok(())
    }

    pub fn start(&self, gpu: &GpuContext) {
        self.clear(gpu);
        unsafe {
            gpu.context().OMSetRenderTargets(
                Some(&[Some(self.pick_buffer.render_target.clone())]),
                Some(&self.outline_depth.view),
            );

            gpu.current_states
                .store(StateSelection::new(Some(0), Some(2), Some(2), Some(0)));
        }
    }

    pub fn end(&self) {
        self.pick_buffer.copy_to_staging(&self.pick_buffer_staging);
    }

    pub fn with_entity(&self, entity: Entity, f: impl FnOnce()) {
        self.set_entity(entity);
        f();
        self.pocus().active_entity = None;
    }

    fn set_entity(&self, entity: Entity) {
        if Some(entity) != self.active_entity {
            self.pocus().active_entity = Some(entity);
            self.pick_cb.write(&entity.id()).ok();
        }
    }

    pub fn clear(&self, gpu: &GpuContext) {
        self.outline_depth.clear(0.0, 0);

        unsafe {
            gpu.context()
                .OMSetRenderTargets(Some(&[Some(self.pick_buffer.render_target.clone())]), None);

            gpu.set_blend_state(0);
            gpu.context().RSSetState(None);
            gpu.set_input_topology(EPrimitiveType::Triangles);
            gpu.context().OMSetDepthStencilState(None, 0);
            gpu.context().VSSetShader(&self.clear_vs, None);
            gpu.context().PSSetShader(&self.clear_ps, None);

            gpu.context().Draw(3, 0);
        }
    }

    pub fn get(&self, x: usize, y: usize) -> u32 {
        self.pick_buffer_staging
            .map(D3D11_MAP_READ, |m| unsafe {
                let data = m
                    .pData
                    .cast::<u8>()
                    .add(y * m.RowPitch as usize + x * size_of::<u32>())
                    .cast::<u32>();

                data.read()
            })
            .unwrap_or(u32::MAX)
    }
}
