use std::any::Any;

use alkahest_data::tfx::{features::dynamic::RenderStageSubscription, RenderStage};
use glam::Mat4;

use crate::{
    camera::Camera, gpu::command_list::CommandList, tfx::packet::CompactTransform, Renderer,
};

pub mod cubemap;
pub mod decals;
pub mod decorators;
pub mod immediate;
pub mod light;
pub mod rigid_model;
mod shared;
pub mod static_geometry;
pub mod terrain_patches;

pub trait FeatureRenderer {
    /// Returns false if the render object should be discarded
    fn visibility_test(&mut self, _camera: &Camera) -> bool {
        true
    }

    // TODO(cohae): Storing the extracted data in the render object seems a bit excessive when the frame node data is guaranteed to be valid for the duration of this call, do we really need it?
    fn extract_and_prepare(&mut self, renderer: &Renderer, extracted_data: &dyn Any);
    fn submit(&self, cmd: &mut CommandList, stage: RenderStage);

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
