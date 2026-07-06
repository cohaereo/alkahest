use std::{any::Any, sync::Arc};

use alkahest_core::job::potassium::JobHandle;
use alkahest_data::tfx::{
    RenderStage, TfxFeatureRenderer, features::dynamic::RenderStageSubscription,
};

use crate::{
    Renderer,
    feature::FeatureRenderer,
    gpu::command_list::CommandList,
    renderer::visibility::OpaqueView,
    util::{arena, threading::CommandListSetId},
};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderObjectHandle(pub(crate) arena::Index);

impl RenderObjectHandle {
    pub const INVALID: Self = Self(arena::Index::INVALID);

    pub fn is_valid(&self) -> bool {
        self != &Self::INVALID
    }

    pub fn index(&self) -> arena::Index {
        self.0
    }
}

impl From<RenderObjectHandle> for arena::Index {
    fn from(handle: RenderObjectHandle) -> Self {
        handle.0
    }
}

pub struct RenderObject {
    renderer: Box<dyn FeatureRenderer>,
    pub feature_type: TfxFeatureRenderer,
    pub stages: RenderStageSubscription,
}

impl RenderObject {
    pub fn new(kind: TfxFeatureRenderer, renderer: Box<dyn FeatureRenderer>) -> Self {
        Self {
            stages: renderer.subscribed_stages(),
            renderer,
            feature_type: kind,
        }
    }

    pub fn dyn_clone(&self) -> Option<Self> {
        Some(Self {
            stages: self.stages,
            renderer: self.renderer.dyn_clone()?,
            feature_type: self.feature_type,
        })
    }

    pub fn get_mut<T: FeatureRenderer + 'static>(&mut self) -> Option<&mut T> {
        self.renderer.as_any_mut().downcast_mut::<T>()
    }
}

impl RenderObject {
    pub fn visibility_test(&mut self, view_index: usize, view: &dyn OpaqueView) -> bool {
        self.renderer.visibility_test(view_index, view)
    }

    pub fn prepare(&mut self, renderer: &Renderer, view_index: usize, data: &dyn Any) {
        self.renderer.prepare(renderer, view_index, data);
    }

    pub fn submit(&self, cmd: &mut CommandList, view_index: usize, stage: RenderStage) {
        self.renderer.submit(cmd, view_index, stage);
    }

    pub fn submit_parallel(
        &self,
        renderer: &Arc<Renderer>,
        view_index: usize,
        set: CommandListSetId,
        stage: RenderStage,
        jobs: &mut Vec<JobHandle>,
    ) {
        self.renderer
            .submit_parallel(renderer, view_index, set, stage, jobs);
    }

    pub fn is_loaded(&self) -> bool {
        self.renderer.is_loaded()
    }
}

unsafe impl Send for RenderObject {}
unsafe impl Sync for RenderObject {}
