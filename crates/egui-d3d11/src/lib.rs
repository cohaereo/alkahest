#![allow(unexpected_cfgs)]

mod mesh;
mod painter;
mod shader;
mod texture;

pub use painter::*;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("Unrecoverable error: {0}")]
    General(&'static str),

    #[error("D3D11 error {0}")]
    D3D11(#[from] d3d11::Error),
}
