use alkahest_data::map::SAudioClipCollection;
use bevy_ecs::prelude::Component;

#[derive(Component)]
pub struct AmbientAudio {
    _data: SAudioClipCollection,
}

impl AmbientAudio {
    pub fn new(data: SAudioClipCollection) -> Self {
        Self { _data: data }
    }
}
