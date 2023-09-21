mod backup;
mod mesh;
mod painter;
mod shader;
mod texture;

pub use painter::*;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("Unrecoverable error occured {0}")]
    General(&'static str),

    #[error("Windows error {0}")]
    Win(#[from] windows::core::Error),
}
