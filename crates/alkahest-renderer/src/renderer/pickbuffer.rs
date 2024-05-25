use std::{
    mem::size_of,
    sync::atomic::{AtomicBool, Ordering},
};

use alkahest_data::{
    dxgi::DxgiFormat,
    geometry::EPrimitiveType,
    technique::StateSelection,
    tfx::{TfxRenderStage, TfxShaderStage},
};
use anyhow::Context;
use crossbeam::atomic::AtomicCell;
use hecs::Entity;
use windows::Win32::{
    Foundation::RECT,
    Graphics::Direct3D11::{
        ID3D11PixelShader, ID3D11RenderTargetView, ID3D11VertexShader, D3D11_MAP_READ,
    },
};

use crate::{
    ecs::{render::draw_entity, Scene},
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
        *self.pickbuffer.selected_entity.pocus() = selected;
        self.run_renderstage_systems(scene, TfxRenderStage::DepthPrepass);
        *self.gpu.custom_pixel_shader.pocus() = None;
        self.pickbuffer.end(&self.gpu);
    }

    pub(super) fn draw_outline(&self, scene: &Scene, selected: Entity) {
        gpu_event!(self.gpu, "selection_outline");

        self.pickbuffer.outline_depth.clear(0.0, 0);

        unsafe {
            const NO_RT: Option<ID3D11RenderTargetView> = None;
            let mut rt_backup = [NO_RT; 4];
            self.gpu
                .context()
                .OMGetRenderTargets(Some(&mut rt_backup), None);

            // Draw the selected entity into the outline depth buffer
            self.gpu
                .context()
                .OMSetRenderTargets(None, Some(&self.pickbuffer.outline_depth.view));
            self.gpu
                .context()
                .OMSetDepthStencilState(Some(&self.pickbuffer.outline_depth.state), 0);
            draw_entity(scene, selected, self, TfxRenderStage::GenerateGbuffer);

            // Draw the outline itself
            self.gpu
                .context()
                .OMSetRenderTargets(Some(&rt_backup), None);
            self.gpu.context().OMSetDepthStencilState(None, 0);

            self.gpu.flush_states();
            self.gpu.set_blend_state(12);
            self.gpu.set_depth_stencil_state(1);
            self.gpu.context().RSSetState(None);
            self.gpu.set_input_topology(EPrimitiveType::Triangles);
            self.gpu
                .context()
                .VSSetShader(&self.pickbuffer.outline_vs, None);
            self.gpu
                .context()
                .PSSetShader(&self.pickbuffer.outline_ps, None);
            self.gpu.context().PSSetShaderResources(
                0,
                Some(&[
                    Some(self.pickbuffer.outline_depth.texture_view.clone()),
                    Some(self.data.lock().gbuffers.depth.texture_view.clone()),
                ]),
            );

            self.gpu.context().Draw(3, 0);
        }
    }
}

pub struct Pickbuffer {
    pub(super) selection_request: AtomicCell<Option<(u32, u32)>>,
    pub outline_depth: DepthState,
    pub pick_buffer: RenderTarget,
    pub pick_buffer_staging: CpuStagingBuffer,

    pub(super) outline_vs: ID3D11VertexShader,
    pub(super) outline_ps: ID3D11PixelShader,

    clear_vs: ID3D11VertexShader,
    clear_ps: ID3D11PixelShader,

    pick_ps: ID3D11PixelShader,
    pick_cb: ConstantBuffer<u32>,
    active_entity: Option<Entity>,
    /// The entity that's already selected. Will not be drawn into the pickbuffer
    selected_entity: Option<Entity>,
}

impl Pickbuffer {
    pub fn new(gctx: SharedGpuContext, window_size: (u32, u32)) -> anyhow::Result<Self> {
        let outline_vs = gctx
            .device
            .load_vertex_shader(include_dxbc!(vs "gui/outline.hlsl"))?;
        let outline_ps = gctx
            .device
            .load_pixel_shader(include_dxbc!(ps "gui/outline.hlsl"))?;

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
            selection_request: AtomicCell::new(None),
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

            outline_vs,
            outline_ps,
            clear_vs,
            clear_ps,
            pick_ps,
            pick_cb: ConstantBuffer::create(gctx.clone(), None)?,
            active_entity: None,
            selected_entity: None,
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

    pub fn request_selection(&self, x: u32, y: u32) {
        self.pocus().selection_request.store(Some((x, y)));
    }

    /// Finish the current selection request and return the entity id at the request coordinates
    /// Must only be called after the current frame has been processed by the GPU
    pub fn finish_request(&self) -> Option<u32> {
        self.pocus()
            .selection_request
            .take()
            .map(|(x, y)| self.get(x as usize, y as usize))
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

            // Limit the draw area to as small as possible
            if let Some((x, y)) = self.selection_request.load() {
                gpu.context().RSSetScissorRects(Some(&[RECT {
                    left: x as i32 - 1,
                    top: y as i32 - 1,
                    right: x as i32 + 1,
                    bottom: y as i32 + 1,
                }]))
            }
        }
    }

    pub fn end(&self, gpu: &GpuContext) {
        self.pick_buffer.copy_to_staging(&self.pick_buffer_staging);
        unsafe {
            gpu.context().RSSetScissorRects(None);
        }
    }

    pub fn with_entity(&self, entity: Entity, f: impl FnOnce()) {
        self.set_entity(entity);
        f();
        self.pocus().active_entity = None;
    }

    fn set_entity(&self, mut entity: Entity) {
        if Some(entity) == self.selected_entity {
            entity = Entity::DANGLING;
        }

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
