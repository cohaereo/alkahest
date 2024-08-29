pub mod black_magic;
pub mod color;
pub mod d3d;
pub mod image;
pub mod math;
pub mod packages;
pub mod scene;
pub mod text;

use std::any::Any;

pub use math::*;

pub fn short_type_name<T: Any>() -> &'static str {
    std::any::type_name::<T>().rsplit("::").next().unwrap()
}

/// Nice immutable reference you got there, would be a shame if someone were to mutate it...
pub trait Hocus {
    /// I'LL STEAL IT, NO ONE WILL EVER KNOW!
    #[allow(clippy::mut_from_ref)]
    fn pocus(&self) -> &mut Self;
}

impl<T> Hocus for T {
    #[allow(invalid_reference_casting, clippy::mut_from_ref)]
    fn pocus(&self) -> &mut T {
        unsafe { &mut *(self as *const _ as *mut _) }
    }
}
