use std::{sync::Arc, time::Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::{
    gpu::SharedGpuContext,
    loaders::AssetManager,
    tfx::{externs::ExternStorage, gbuffer::GBuffer, globals::RenderGlobals},
};
use crate::handle::Handle;
use crate::tfx::technique::Technique;

pub type RendererShared = Arc<Renderer>;

pub struct Renderer {
    pub gpu: SharedGpuContext,

    pub render_globals: RenderGlobals,
    pub data: Mutex<RendererData>,

    pub time: Instant,
}

pub struct RendererData {
    pub asset_manager: AssetManager,
    pub gbuffers: GBuffer,
    pub externs: ExternStorage,
}

impl Renderer {
    pub fn create(gpu: SharedGpuContext, window_size: (u32, u32)) -> anyhow::Result<Arc<Self>> {
        let render_globals =
            RenderGlobals::load(gpu.clone()).expect("Failed to load render globals");

        Ok(Arc::new(Self {
            data: Mutex::new(RendererData {
                asset_manager: AssetManager::new(gpu.clone()),
                gbuffers: GBuffer::create(window_size, gpu.clone())?,
                externs: ExternStorage::default(),
            }),
            gpu,
            render_globals,
            time: Instant::now(),
        }))
    }
    
    pub fn get_technique_shared(&self, handle: &Handle<Technique>) -> Option<Arc<Technique>> {
        let data = self.data.lock();
        data.asset_manager.techniques.get_shared(handle)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RendererSettings {
    pub vsync: bool,
    pub ssao: bool,
    pub atmosphere: bool,
    pub matcap: bool,
}

impl Default for RendererSettings {
    fn default() -> Self {
        Self {
            vsync: true,
            ssao: true,
            atmosphere: false,
            matcap: false,
        }
    }
}
