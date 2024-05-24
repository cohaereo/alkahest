use hecs::Entity;
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

        self.insert_one(child, Parent(parent)).unwrap();
        if let Ok(mut children) = self.get::<&mut Children>(parent) {
            children.0.push(child);
            return;
        }
        
        self.insert_one(parent, Children(smallvec![child])).unwrap();
    }

    fn get_parent(&self, child: Entity) -> Option<Entity> {
        self.get::<&Parent>(child).ok().map(|p| p.0)
    }
}
