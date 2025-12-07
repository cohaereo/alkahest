use lazy_static::lazy_static;
use parking_lot::Mutex;
use renderdoc::{RenderDoc, V100};

pub mod arena;
pub mod byteutil;
pub mod d3d;
pub mod fps_histogram;
pub mod geometry;
pub mod math;
pub mod threading;

lazy_static! {
    pub static ref RENDERDOC: Option<Mutex<RenderDoc<V100>>> =
        RenderDoc::new().ok().map(Mutex::new);
}

pub fn is_renderdoc_connected() -> bool {
    RENDERDOC
        .as_ref()
        .is_some_and(|r| r.lock().is_remote_access_connected())
}
