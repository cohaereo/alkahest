pub mod color;
pub mod d3d;
pub mod image;
pub mod math;
pub mod packages;

use std::any::Any;

pub use math::*;

pub fn short_type_name<T: Any>() -> &'static str {
    std::any::type_name::<T>().rsplit("::").next().unwrap()
}

/// Nice immutable reference you got there, would be a shame if someone were to mutate it...
pub struct Hocus<'a, T>(pub &'a T);
impl<T> Hocus<'_, T> {
    /// I'LL STEAL IT, NO ONE WILL EVER KNOW!
    #[allow(invalid_reference_casting, clippy::mut_from_ref)]
    pub fn pocus(&self) -> &mut T {
        unsafe { &mut *(self.0 as *const _ as *mut _) }
    }
}

#[macro_export]
/// Gets a mutable reference to a field of a struct on an immutable instance.
/// This is unsafe and should only be used where we know we're the only one accessing the field.
macro_rules! hocus {
    ($var:expr) => {
        $crate::util::Hocus($var).pocus()
    };
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