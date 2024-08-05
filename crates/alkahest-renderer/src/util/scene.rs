use bevy_ecs::{component::Component, entity::Entity, prelude::EntityWorldMut};
use smallvec::smallvec;

use crate::ecs::{
    hierarchy::{Children, Parent},
    Scene,
};

pub trait SceneExt {
    fn set_parent(&mut self, child: Entity, parent: Entity);
    fn get_parent(&self, child: Entity) -> Option<Entity>;
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
}

pub trait EntityWorldMutExt {
    fn insert_one<T: Component>(&mut self, component: T) -> &mut Self;
}

impl<'w> EntityWorldMutExt for EntityWorldMut<'w> {
    fn insert_one<T: Component>(&mut self, component: T) -> &mut Self {
        self.insert((component,))
    }
}
