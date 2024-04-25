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

    pub fn get<T: Any>(&self) -> Ref<'_, T> {
        match self.resources.get(&TypeId::of::<T>()).map(|resource| {
            Ref::map(
                match resource.try_borrow() {
                    Ok(r) => r,
                    Err(e) => panic!(
                        "Failed to get reference to resource {}: {e}",
                        std::any::type_name::<T>()
                    ),
                },
                |r| r.downcast_ref::<T>().unwrap(),
            )
        }) {
            Some(r) => r,
            None => panic!("Resource not found: {}", std::any::type_name::<T>()),
        }
    }

    pub fn get_mut<T: Any>(&self) -> RefMut<'_, T> {
        match self.resources.get(&TypeId::of::<T>()).map(|resource| {
            RefMut::map(
                match resource.try_borrow_mut() {
                    Ok(r) => r,
                    Err(e) => panic!(
                        "Failed to get mutable reference to resource {}: {e}",
                        std::any::type_name::<T>()
                    ),
                },
                |r| r.downcast_mut::<T>().unwrap(),
            )
        }) {
            Some(r) => r,
            None => panic!("Resource not found: {}", std::any::type_name::<T>()),
        }
    }
}
