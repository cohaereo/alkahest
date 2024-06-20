use alkahest_data::map::SAudioClipCollection;

pub struct AmbientAudio {
    data: SAudioClipCollection,
}

impl AmbientAudio {
    pub fn new(data: SAudioClipCollection) -> Self {
        Self { data }
    }
}
