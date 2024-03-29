use std::time::Instant;

use hecs::Entity;

pub struct SelectedEntity(
    pub Option<Entity>,
    /// has an entity been selected this frame?
    pub bool,
    pub Instant,
);
