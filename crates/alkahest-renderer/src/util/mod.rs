pub mod color;
pub mod d3d;
pub mod image;
pub mod math;
pub mod packages;
pub mod scene;

use std::any::Any;

pub use math::*;

pub fn short_type_name<T: Any>() -> &'static str {
    std::any::type_name::<T>().rsplit("::").next().unwrap()
}

/// Nice immutable reference you got there, would be a shame if someone were to mutate it...
pub trait Hocus {
    /// I'LL STEAL IT, NO ONE WILL EVER KNOW!
    fn pocus(&self) -> &mut Self;
}

impl<T> Hocus for T {
    #[allow(invalid_reference_casting, clippy::mut_from_ref)]
    fn pocus(&self) -> &mut T {
        unsafe { &mut *(self as *const _ as *mut _) }
    }
}

pub trait StringExt {
    fn truncate_ellipsis(&self, max_len: usize) -> String;
}

impl StringExt for String {
    fn truncate_ellipsis(&self, max_len: usize) -> String {
        if self.len() > max_len {
            format!("{}...", &self[..max_len - 3])
        } else {
            self.clone()
        }
    }
}
