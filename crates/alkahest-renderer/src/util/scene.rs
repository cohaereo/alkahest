use bevy_ecs::{
    component::{BoxedComponent, Component},
    entity::Entity,
    prelude::EntityWorldMut,
};
use itertools::Itertools;
use smallvec::smallvec;

use crate::ecs::{
    hierarchy::{Children, Parent},
    Scene,
};

pub trait SceneExt {
    fn set_parent(&mut self, child: Entity, parent: Entity);
    fn get_parent(&self, child: Entity) -> Option<Entity>;

    fn take_boxed(&mut self, entity: Entity) -> Option<Vec<BoxedComponent>>;
    fn spawn_boxed(&mut self, components: impl IntoIterator<Item = BoxedComponent>) -> Entity;
}

impl SceneExt for Scene {
    fn set_parent(&mut self, child: Entity, parent: Entity) {
        let original_parent = self.get_parent(child);
        if original_parent == Some(parent) {
            return;
        }

        self.entity_mut(child).insert_one(Parent(parent));
        if let Some(mut children) = self.entity_mut(parent).get_mut::<Children>() {
            children.0.push(child);
            return;
        }

        self.entity_mut(parent)
            .insert_one(Children(smallvec![child]));
    }

    fn get_parent(&self, child: Entity) -> Option<Entity> {
        self.get::<Parent>(child).map(|parent| parent.0)
    }

    fn take_boxed(&mut self, entity: Entity) -> Option<Vec<BoxedComponent>> {
        let mut er = self.get_entity_mut(entity)?;
        let component_ids = er.archetype().components().collect_vec();
        let mut components = vec![];
        for c in component_ids {
            let component = unsafe { er.take_by_id(c) }.unwrap();
            components.push(component);
        }

        er.despawn();

        Some(components)
    }

    fn spawn_boxed(&mut self, components: impl IntoIterator<Item = BoxedComponent>) -> Entity {
        let mut new_component_ids = vec![];
        for c in components.into_iter() {
            let new_component_id = self.init_component_with_descriptor(c.descriptor().clone());

            new_component_ids.push((new_component_id, c));
        }

        let mut new_entity = self.spawn_empty();
        for (new_component_id, component) in new_component_ids {
            unsafe {
                new_entity.insert_by_id(new_component_id, component.to_ptr());
            }
        }

        new_entity.id()
    }
}

pub trait EntityWorldMutExt {
    fn insert_one<T: Component>(&mut self, component: T) -> &mut Self;
}

impl<'w> EntityWorldMutExt for EntityWorldMut<'w> {
    fn insert_one<T: Component>(&mut self, component: T) -> &mut Self {
        self.insert((component,))
    }
}
