use std::time::Instant;

use hecs::Entity;

pub struct SelectedEntity {
    selected: Option<Entity>,
    /// Has an entity been selected this frame?
    pub changed_this_frame: bool,
    /// Time the entity was selected
    pub time_selected: Instant,
}

impl Default for SelectedEntity {
    fn default() -> Self {
        Self {
            selected: None,
            changed_this_frame: false,
            time_selected: Instant::now(),
        }
    }
}

impl SelectedEntity {
    pub fn select(&mut self, entity: Entity) {
        self.selected = Some(entity);
        self.changed_this_frame = true;
        self.time_selected = Instant::now();
    }

    pub fn deselect(&mut self) {
        self.selected = None;
        self.changed_this_frame = true;
        self.time_selected = Instant::now();
    }

    pub fn selected(&self) -> Option<Entity> {
        self.selected
    }
}
