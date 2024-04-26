pub mod d3d;
pub mod image;
pub mod math;

use std::any::Any;

pub use math::*;

pub fn short_type_name<T: Any>() -> &'static str {
    std::any::type_name::<T>().rsplit("::").next().unwrap()
}
