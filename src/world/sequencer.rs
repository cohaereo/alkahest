use alkahest_render::{Renderer, tfx::sequencer_vm::global_channel::GlobalChannelExpression};
use egui::ahash::HashSet;

pub fn s_evaluate_global_channel_expressions(world: &hecs::World) {
    for (_entity, gce) in world.query::<&mut GlobalChannelExpression>().iter() {
        gce.evaluate(Renderer::instance().externs.get_mut());
    }
}

pub fn s_get_all_global_channel_ids(world: &hecs::World) -> HashSet<u32> {
    let mut ids = HashSet::default();
    for (_entity, gce) in world.query::<&GlobalChannelExpression>().iter() {
        ids.insert(gce.channel_id);
    }

    ids
}
