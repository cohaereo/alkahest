use std::mem::size_of;

use alkahest_data::{dxgi::DxgiFormat, geometry::EPrimitiveType};
use anyhow::Context;
use hecs::Entity;
use windows::Win32::Graphics::Direct3D11::{ID3D11PixelShader, ID3D11VertexShader, D3D11_MAP_READ};

use crate::{
    ecs::Scene,
    gpu::{buffer::ConstantBuffer, util::DxDeviceExt, GpuContext, SharedGpuContext},
    hocus, include_dxbc,
    renderer::{
        gbuffer::{CpuStagingBuffer, DepthState, RenderTarget},
        Renderer,
    },
};

impl Renderer {
    pub(super) fn draw_pickbuffer(&self, scene: &Scene, selected: Option<Entity>) {
        //
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

    pub fn resize(&self, mut new_size: (u32, u32)) -> anyhow::Result<()> {
        if new_size.0 == 0 || new_size.1 == 0 {
            new_size = (1, 1);
        }

        hocus!(self)
            .outline_depth
            .resize(new_size)
            .context("Outline Depth")?;

        hocus!(self)
            .pick_buffer
            .resize(new_size)
            .context("Entity_Pickbuffer")?;
        hocus!(self)
            .pick_buffer_staging
            .resize(new_size)
            .context("Entity_Pickbuffer_Staging")?;

        Ok(())
    }

    pub fn start(&self, gpu: &GpuContext) {
        self.clear(gpu);
    }

    pub fn with_entity(&self, entity: Entity, f: impl FnOnce()) {
        self.set_entity(entity);
        f();
        hocus!(self).active_entity = None;
    }

    fn set_entity(&self, entity: Entity) {
        if Some(entity) != self.active_entity {
            hocus!(self).active_entity = Some(entity);
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
