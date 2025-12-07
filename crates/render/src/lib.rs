#![feature(allocator_api, array_try_from_fn)]

pub mod gpu;
pub use gpu::Gpu;
pub mod asset;
pub mod feature;
pub mod renderer;
pub use renderer::Renderer;
pub mod camera;
pub mod object;
pub mod tfx;
pub mod util;
pub mod visibility;

#[macro_use]
extern crate tracing;

#[macro_export]
macro_rules! gpu_span {
    () => {};
}
