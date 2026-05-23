use glam::Vec3;

use crate::{
    audio::{AudioSource, LISTENER_ID},
    world::transform::Transform,
};

const MAX_DISTANCE: f32 = 30.0;
const MIN_VOLUME: f32 = 0.0;

pub fn s_update_audio_sources(world: &hecs::World, listener_pos: Vec3) {
    profiling::scope!("update_audio_positions");

    for (_entity, (transform, audio_source)) in
        world.query::<(Option<&Transform>, &AudioSource)>().iter()
    {
        let position = transform.map(|t| t.translation).unwrap_or_default();
        let distance = position.distance(listener_pos);
        let volume = (1.0 - (distance / MAX_DISTANCE).clamp(0.0, 1.0)).max(MIN_VOLUME);

        audio_source.set_position(position);
        rrise::sound_engine::set_game_object_output_bus_volume(
            audio_source.gameobject_id,
            LISTENER_ID,
            volume,
        );
    }
}
