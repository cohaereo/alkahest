use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::{
    gpu::SharedGpuContext,
    loaders::AssetManager,
    tfx::{externs::ExternStorage, gbuffer::GBuffer},
};

pub struct Renderer {
    pub gpu: SharedGpuContext,
    pub data: Mutex<RendererData>,
}

pub struct RendererData {
    pub asset_manager: AssetManager,
    pub gbuffers: GBuffer,
    pub externs: ExternStorage,
}

impl Renderer {
    pub fn new(gpu: SharedGpuContext) -> anyhow::Result<Self> {
        Ok(Self {
            data: Mutex::new(RendererData {
                asset_manager: AssetManager::new(gpu.clone()),
                gbuffers: GBuffer::create((4, 4), gpu.clone())?,
                externs: ExternStorage::default(),
            }),
            gpu,
        })
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
            atmosphere: true,
            matcap: false,
        }
    }
}
