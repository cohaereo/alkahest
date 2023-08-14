use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell, RefMut},
    mem::transmute,
};

use nohash_hasher::IntMap;

#[derive(Default)]
pub struct Resources {
    resources: IntMap<u64, RefCell<Box<dyn Any>>>,
}

impl Resources {
    pub fn insert<T: Any>(&mut self, v: T) {
        // Safety: TypeId is a u64
        let type_id: u64 = unsafe { transmute(TypeId::of::<T>()) };
        self.resources.insert(type_id, RefCell::new(Box::new(v)));
    }

    pub fn get<T: Any>(&self) -> Option<Ref<T>> {
        // Safety: TypeId is a u64
        let type_id: u64 = unsafe { transmute(TypeId::of::<T>()) };
        self.resources
            .get(&type_id)
            .map(|resource| Ref::map(resource.borrow(), |r| r.downcast_ref::<T>().unwrap()))
    }

    pub fn get_mut<T: Any>(&self) -> Option<RefMut<T>> {
        // Safety: TypeId is a u64
        let type_id: u64 = unsafe { transmute(TypeId::of::<T>()) };
        self.resources
            .get(&type_id)
            .map(|resource| RefMut::map(resource.borrow_mut(), |r| r.downcast_mut::<T>().unwrap()))
    }
}
