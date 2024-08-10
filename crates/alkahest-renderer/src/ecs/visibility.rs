use std::process::Child;

use bevy_ecs::{
    bundle::Bundle,
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    query::{With, Without},
    system::Query,
    world::Ref,
};

use super::hierarchy::{Children, Parent};
use crate::util::Hocus;

#[derive(Bundle, Default)]
pub struct VisibilityBundle {
    pub visibility: Visibility,
    pub view_visibility: ViewVisibility,
}

/// Describe the visibility of an entity
///
/// ⚠ If you need to query the visibility of an entity for rendering purposes, use `ViewVisibility` instead
#[derive(Component, Copy, Clone, PartialEq, Default, Debug)]
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

    /// Compare this visibility with that of a child, returning a value if the child visibility should be updated to that value
    pub fn compare(&self, other: &Self) -> Option<Self> {
        if self == other || self == &Visibility::Hidden && other == &Visibility::InheritedHidden {
            return None;
        }

        match other {
            Visibility::Visible | Visibility::InheritedHidden => {
                if self.is_visible() {
                    Some(Visibility::Visible)
                } else {
                    Some(Visibility::InheritedHidden)
                }
            }
            Visibility::Hidden => None,
        }
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

pub fn propagate_entity_visibility_system(
    q_root: Query<(&Children, Option<&Visibility>), Without<Parent>>,
    q_visibility: Query<(Option<&Children>, Option<&Visibility>), With<Parent>>,
) {
    puffin::profile_function!();

    for (children, vis) in q_root.iter() {
        let vis = vis.cloned().unwrap_or_default();
        for child in children.iter() {
            propagate_entity_visibility_recursive(*child, vis, &q_visibility);
        }
    }
}

fn propagate_entity_visibility_recursive(
    entity: Entity,
    parent_visibility: Visibility,
    q_visibility: &Query<(Option<&Children>, Option<&Visibility>), With<Parent>>,
) {
    let Ok((children, vis)) = q_visibility.get(entity) else {
        return;
    };

    if let Some(vis) = vis.as_ref() {
        if let Some(new_vis) = parent_visibility.compare(vis) {
            *(*vis).pocus() = new_vis;
        }
    }

    let vis = vis.map(|v| v.clone()).unwrap_or_default();
    if let Some(children) = &children {
        for child in children.iter() {
            propagate_entity_visibility_recursive(*child, vis, q_visibility);
        }
    }
}

pub fn calculate_view_visibility_system(
    mut q_visibility: Query<(Option<&Visibility>, &mut ViewVisibility)>,
) {
    puffin::profile_function!();
    for (vis, mut view_vis) in q_visibility.iter_mut() {
        if vis.is_visible() {
            view_vis.set();
        } else {
            view_vis.reset();
        }
    }
}
