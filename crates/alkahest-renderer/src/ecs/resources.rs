use std::time::Instant;

use hecs::Entity;

pub struct SelectedEntity {
    pub select: Option<Entity>,
    /// Has an entity been selected this frame?
    pub changed_this_frame: bool,
    /// Time the entity was selected
    pub time_selected: Instant,
}
