use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell, RefMut},
};

use nohash_hasher::IntMap;

macro_rules! raw_typeid {
    ($t:ty) => {
        // Safety: see TypeId::hash
        unsafe { ((&TypeId::of::<T>() as *const TypeId) as *const u64).read() }
    };
}

#[derive(Default)]
pub struct Resources {
    resources: IntMap<u64, RefCell<Box<dyn Any>>>,
}

impl Resources {
    pub fn insert<T: Any>(&mut self, v: T) {
        let type_id: u64 = raw_typeid!(T);
        self.resources.insert(type_id, RefCell::new(Box::new(v)));
    }

    pub fn get<T: Any>(&self) -> Option<Ref<'_, T>> {
        let type_id: u64 = raw_typeid!(T);
        self.resources
            .get(&type_id)
            .map(|resource| Ref::map(resource.borrow(), |r| r.downcast_ref::<T>().unwrap()))
    }

    pub fn get_mut<T: Any>(&self) -> Option<RefMut<'_, T>> {
        let type_id: u64 = raw_typeid!(T);
        self.resources
            .get(&type_id)
            .map(|resource| RefMut::map(resource.borrow_mut(), |r| r.downcast_mut::<T>().unwrap()))
    }
}
