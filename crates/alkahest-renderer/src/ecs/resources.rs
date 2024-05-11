use std::time::Instant;

use hecs::Entity;

use crate::{
    camera::tween::ease_out_exponential,
    util::color::{Color, ColorExt},
};

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

    pub fn select_fade_color(&self, base_color: Color, entity: Option<Entity>) -> Color {
        let select_color = Color::from_rgb(1.0, 0.6, 0.2);
        let elapsed =
            ease_out_exponential((self.time_selected.elapsed().as_secs_f32() / 1.4).min(1.0));

        if self.selected() == entity && elapsed < 1.0 {
            let c = select_color.to_vec4().lerp(base_color.to_vec4(), elapsed);
            Color::from_rgb(c.x, c.y, c.z)
        } else {
            base_color
        }
    }
}
