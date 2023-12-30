pub mod component_panels;
pub mod components;
pub mod resources;
pub mod tags;
pub mod transform;

pub type Scene = hecs::World;

use hecs::EntityRef;

use crate::ecs::component_panels::ComponentPanel;
use crate::ecs::components::*;

pub fn resolve_entity_icon(e: EntityRef<'_>) -> Option<char> {
    macro_rules! icon_from_component_panels {
		($($component:ty),+) => {
			$(
				if e.has::<$component>() {
					return Some(<$component>::inspector_icon());
				}
			)*
		};
	}

    icon_from_component_panels!(
        // TODO(cohae): Custom havok icon
        // HavokShape,
        Ruler,
        EntityModel,
        StaticInstances,
        ResourcePoint
    );

    None
}

pub fn resolve_entity_name(e: EntityRef<'_>) -> String {
    if let Some(label) = e.get::<&Label>() {
        format!("{} (ent {})", label.0, e.entity().id())
    } else {
        format!("ent {}", e.entity().id())
    }
}
