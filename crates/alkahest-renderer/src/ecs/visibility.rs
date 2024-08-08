use bevy_ecs::{bundle::Bundle, component::Component, entity::Entity, system::Query, world::Ref};

#[derive(Bundle, Default)]
pub struct VisibilityBundle {
    pub visibility: Visibility,
    pub view_visibility: ViewVisibility,
}

/// Describe the visibility of an entity
///
/// ⚠ If you need to query the visibility of an entity for rendering purposes, use `ViewVisibility` instead
#[derive(Component, Copy, Clone, PartialEq, Default)]
pub enum Visibility {
    /// The entity is explicitly marked as visible
    #[default]
    Visible,
    /// One of the entity's ancestors is hidden
    InheritedHidden,
    /// The entity is explicitly marked as hidden
    Hidden,
}

impl Visibility {
    pub fn is_visible(&self) -> bool {
        self == &Visibility::Visible
    }
}

/// Describe the visibility of an entity for the current view (eg. an entity may be hidden due to frustum culling, but not explicitly marked as hidden)
#[derive(Component, Copy, Clone, PartialEq, Default)]
pub struct ViewVisibility(bool);

impl ViewVisibility {
    pub fn is_visible(&self) -> bool {
        self.0
    }

    pub fn set(&mut self) {
        self.0 = true;
    }

    pub fn reset(&mut self) {
        self.0 = false;
    }
}

pub trait VisibilityHelper {
    fn is_visible(&self) -> bool;
}

impl VisibilityHelper for Option<&Visibility> {
    fn is_visible(&self) -> bool {
        self.map_or(true, |v| v.is_visible())
    }
}

impl VisibilityHelper for Option<&ViewVisibility> {
    fn is_visible(&self) -> bool {
        self.map_or(true, |v| v.is_visible())
    }
}

pub fn propagate_entity_visibility_system(q_entities: Query<(Entity, Option<&Visibility>)>) {}

pub fn calculate_view_visibility_system(
    mut q_entities: Query<(Option<&Visibility>, &mut ViewVisibility)>,
) {
    for (vis, mut view_vis) in q_entities.iter_mut() {
        if vis.is_visible() {
            view_vis.set();
        } else {
            view_vis.reset();
        }
    }
}
