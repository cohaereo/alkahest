use std::sync::Arc;

use alkahest_data::tfx::{FeatureRendererSubscription, common::AxisAlignedBBox};
use glam::Vec3;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

use crate::{Renderer, tfx::view::View, visibility::frustum::Frustum};

impl Renderer {
    pub fn cull_view(self: &Arc<Self>, view_index: usize, view: &dyn OpaqueView) {
        if view_index >= 32 {
            error!("View index out of range ({}, max 32)", view_index);
            return;
        }

        let features = self.frame_packet.read().misc.subscribed_features;
        // parallel_iter(&mut self.frame_packet.write().frame_nodes, |node| {
        self.frame_packet
            .write()
            .frame_nodes
            .par_iter_mut()
            .for_each(|node| {
                let p = self.objects.data_ptr();
                // SAFETY: We have exclusive access to the frame packet and the objects data, and each render object only has one frame node
                unsafe {
                    if let Some(render_object) = (*p).get_mut(node.render_object_handle.into()) {
                        if !features.is_subscribed(render_object.feature_type) {
                            node.visible.set(view_index, false);
                        } else {
                            node.visible
                                .set(view_index, render_object.visibility_test(view_index, view));
                        }
                    }
                }
            });
    }
}

pub trait OpaqueView: Sync {
    fn is_visible(&self, aabb: &AxisAlignedBBox) -> bool;
    fn position(&self) -> Vec3;
    fn subscribed_features(&self) -> FeatureRendererSubscription;
}

impl OpaqueView for View {
    fn is_visible(&self, aabb: &AxisAlignedBBox) -> bool {
        self.is_visible(aabb)
    }

    fn position(&self) -> Vec3 {
        self.position
    }

    fn subscribed_features(&self) -> FeatureRendererSubscription {
        self.subscribed_features
    }
}

impl OpaqueView for Frustum {
    fn is_visible(&self, aabb: &AxisAlignedBBox) -> bool {
        self.aabb_intersecting(aabb)
    }

    fn position(&self) -> Vec3 {
        self.center()
    }

    fn subscribed_features(&self) -> FeatureRendererSubscription {
        FeatureRendererSubscription::all()
    }
}

pub struct NoCulling;

impl OpaqueView for NoCulling {
    fn is_visible(&self, _aabb: &AxisAlignedBBox) -> bool {
        true
    }

    fn position(&self) -> Vec3 {
        Vec3::ZERO
    }

    fn subscribed_features(&self) -> FeatureRendererSubscription {
        FeatureRendererSubscription::all()
    }
}
