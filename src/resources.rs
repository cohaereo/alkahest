use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell, RefMut},
};

use rustc_hash::FxHashMap;

#[derive(Default)]
pub struct Resources {
    resources: FxHashMap<TypeId, RefCell<Box<dyn Any>>>,
}

impl Resources {
    pub fn insert<T: Any>(&mut self, v: T) {
        self.resources
            .insert(TypeId::of::<T>(), RefCell::new(Box::new(v)));
    }

    pub fn get<T: Any>(&self) -> Option<Ref<'_, T>> {
        self.resources
            .get(&TypeId::of::<T>())
            .map(|resource| Ref::map(resource.borrow(), |r| r.downcast_ref::<T>().unwrap()))
    }

    pub fn get_mut<T: Any>(&self) -> Option<RefMut<'_, T>> {
        self.resources
            .get(&TypeId::of::<T>())
            .map(|resource| RefMut::map(resource.borrow_mut(), |r| r.downcast_mut::<T>().unwrap()))
    }
}
