use std::mem::transmute;

use bevy_ecs::{
    component::Component,
    world::{EntityRef, Mut},
};

pub trait EntityRefDarkMagic {
    fn get_mut<T: Component>(&self) -> Option<Mut<'_, T>>;
}

impl<'e> EntityRefDarkMagic for EntityRef<'e> {
    #[inline]
    fn get_mut<T: Component>(&self) -> Option<Mut<'e, T>> {
        if let Some(r) = self.get_ref::<T>() {
            unsafe { transmute(r) }
        } else {
            None
        }
    }
}
