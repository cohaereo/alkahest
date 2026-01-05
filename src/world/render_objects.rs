use std::sync::Arc;

use alkahest_data::tfx::features::ao::SStaticAmbientOcclusion;
use alkahest_render::{
    Renderer,
    asset::{Handle, vertex_buffer::VertexBuffer},
    object::RenderObjectHandle,
    renderer::surface::Surface,
    tfx::packet::FramePacket,
};

use crate::world::{permutations::PermutationConfig, transform::Transform};

pub struct StaticRenderObject {
    handle: RenderObjectHandle,
}

impl StaticRenderObject {
    pub fn new(render_object: RenderObjectHandle) -> Self {
        Self {
            handle: render_object,
        }
    }
}

impl Drop for StaticRenderObject {
    fn drop(&mut self) {
        Renderer::instance().remove_object(self.handle);
    }
}

pub struct DynamicRenderObject {
    handle: RenderObjectHandle,
    pub permutation: usize,
}

impl DynamicRenderObject {
    pub fn new(render_object: RenderObjectHandle) -> Self {
        Self {
            handle: render_object,
            permutation: 0,
        }
    }
}

impl Drop for DynamicRenderObject {
    fn drop(&mut self) {
        Renderer::instance().remove_object(self.handle);
    }
}

pub struct StaticAmbientOcclusion {
    pub buffer: Handle<VertexBuffer>,
    pub ao: SStaticAmbientOcclusion,
}

impl StaticAmbientOcclusion {
    pub fn new(ao: SStaticAmbientOcclusion) -> Self {
        let buffer = Renderer::instance().asset_manager.load(ao.ao0.buffer);
        Self { buffer, ao }
    }
}

pub fn s_extract_ambient_occlusion(world: &hecs::World) {
    let renderer = Renderer::instance();
    if let Some((_entity, ao)) = world.query::<&StaticAmbientOcclusion>().iter().next() {
        *renderer.ao.write() = Some(ao.ao.clone());
        *renderer.ao_buffer.write() = Some(ao.buffer.clone());
    }
}

pub fn s_extract_render_objects(world: &hecs::World, frame_packet: &mut FramePacket) {
    for (_entity, static_render_object) in world.query::<&StaticRenderObject>().iter() {
        frame_packet.push_static_render_object(static_render_object.handle);
    }

    for (_entity, (transform, render_object, permutations)) in world
        .query::<(
            Option<&Transform>,
            &DynamicRenderObject,
            Option<&PermutationConfig>,
        )>()
        .iter()
    {
        let transform = transform.copied().unwrap_or_default();
        let permutation = if let Some(permutation) = permutations {
            permutation
                .calculate_permutation_index()
                .unwrap_or(render_object.permutation)
        } else {
            render_object.permutation
        };

        frame_packet.push_dynamic_render_object(
            render_object.handle,
            transform.local_to_world().into(),
            permutation,
        );
    }
}

pub fn s_are_all_objects_loaded(world: &hecs::World, renderer: &Renderer) -> bool {
    for (_entity, static_render_object) in world.query::<&StaticRenderObject>().iter() {
        if !renderer.is_object_loaded(static_render_object.handle) {
            return false;
        }
    }

    for (_entity, render_object) in world.query::<&DynamicRenderObject>().iter() {
        if !renderer.is_object_loaded(render_object.handle) {
            return false;
        }
    }

    true
}
