use alkahest_render::{Renderer, tfx::sequencer_vm::global_channel::GlobalChannelExpression};

pub fn s_evaluate_global_channel_expressions(world: &hecs::World) {
    for (_entity, gce) in world.query::<&mut GlobalChannelExpression>().iter() {
        gce.evaluate(Renderer::instance().externs.get_mut());
    }
}
