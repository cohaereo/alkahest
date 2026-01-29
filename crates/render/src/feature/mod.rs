use std::{any::Any, sync::Arc};

use alkahest_core::job::potassium::JobHandle;
use alkahest_data::tfx::{RenderStage, features::dynamic::RenderStageSubscription};
use glam::Mat4;

use crate::{
    Renderer,
    gpu::command_list::CommandList,
    tfx::{packet::CompactTransform, view::View},
    util::threading::CommandListSetId,
};

pub mod cubemap;
pub mod decals;
pub mod decorators;
pub mod immediate;
pub mod light;
pub mod rigid_model;
pub mod road_decals;
mod shared;
pub mod static_geometry;
pub mod terrain_patches;

pub trait FeatureRenderer {
    /// Returns false if the render object should be discarded
    fn visibility_test(&mut self, view_index: usize, view: &View) -> bool {
        _ = (view, view_index);
        true
    }

    fn prepare(&mut self, renderer: &Renderer, view_index: usize, extracted_data: &dyn Any);

    fn submit(&self, cmd: &mut CommandList, view_index: usize, stage: RenderStage);

    fn submit_parallel(
        &self,
        renderer: &Arc<Renderer>,
        view_index: usize,
        set: CommandListSetId,
        stage: RenderStage,
        jobs: &mut Vec<JobHandle>,
    ) {
        _ = (renderer, view_index, set, stage, jobs);
    }

    fn dyn_clone(&self) -> Option<Box<dyn FeatureRenderer>> {
        None
    }

    fn subscribed_stages(&self) -> RenderStageSubscription;

    /// Returns true if the feature renderer has finished loading any dependencies (techniques, buffers, etc)
    fn is_loaded(&self) -> bool {
        true
    }
}

pub trait FeatureRendererData: Any {
    fn dyn_clone(&self) -> Option<Box<dyn FeatureRendererData>> {
        None
    }
}

macro_rules! impl_feature_renderer_data {
    ($($t:ty),*) => {
        $(
            impl FeatureRendererData for $t {
                fn dyn_clone(&self) -> Option<Box<dyn FeatureRendererData>> {
                    Some(Box::new(self.clone()))
                }
            }
        )*
    };
}

impl_feature_renderer_data!((), Mat4, CompactTransform);
