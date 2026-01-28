use std::sync::Arc;

use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

use crate::{Renderer, tfx::view::View};

impl Renderer {
    pub fn cull_frame_packet(self: &Arc<Self>, view: &View) {
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
                        if !view
                            .subscribed_features
                            .is_subscribed(render_object.feature_type)
                        {
                            node.visible = false;
                        } else {
                            node.visible = render_object.visibility_test(view);
                        }
                    }
                }
            });

        self.frame_packet
            .write()
            .frame_nodes
            .retain(|node| node.visible);
    }
}
