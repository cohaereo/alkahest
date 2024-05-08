use std::ops::Deref;

use hecs::Entity;
use smallvec::SmallVec;

pub struct Parent(pub Entity);
#[derive(Clone)]
pub struct Children(pub SmallVec<[Entity; 8]>);

impl Children {
    pub fn from_slice(slice: &[Entity]) -> Self {
        Self(SmallVec::from_slice(slice))
    }
}

impl Deref for Children {
    type Target = [Entity];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}
