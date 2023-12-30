pub mod component_panels;
pub mod components;
pub mod resources;
pub mod tags;
pub mod transform;

pub type Scene = hecs::World;

use hecs::EntityRef;

use crate::ecs::component_panels::ComponentPanel;
use crate::ecs::components::*;
use crate::util::text::split_pascal_case;

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

    if let Some(rp) = e.get::<&ResourcePoint>() {
        return Some(rp.resource.debug_icon());
    }

    icon_from_component_panels!(
        // TODO(cohae): Custom havok icon
        // HavokShape,
        Ruler,
        EntityModel,
        StaticInstances
    );

    None
}

pub fn resolve_entity_name(e: EntityRef<'_>) -> String {
    let postfix = format!(" (ent {})", e.entity().id());
    if let Some(label) = e.get::<&Label>() {
        format!("{}{postfix}", label.0)
    } else if let Some(rp) = e.get::<&ResourcePoint>() {
        format!("{}{postfix}", split_pascal_case(rp.resource.debug_id()))
    } else {
        format!("ent {}", e.entity().id())
    }
}
