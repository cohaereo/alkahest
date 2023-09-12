use anyhow::Context;
use crossbeam::channel as mpsc;
use destiny_pkg::TagHash;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Instant;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC, D3D11_SUBRESOURCE_DATA,
    D3D11_USAGE_IMMUTABLE,
};

use crate::{
    dxgi::DxgiFormat,
    entity::{IndexBufferHeader, VertexBufferHeader},
    packages::package_manager,
    texture::Texture,
};

use super::{DeviceContextSwapchain, RenderData};

#[derive(PartialEq, Eq, Clone)]
pub enum LoadingThreadState {
    Idle,
    Loading {
        start_time: Instant,
        remaining: usize,
    },
}

pub static STATUS_TEXTURES: RwLock<LoadingThreadState> = RwLock::new(LoadingThreadState::Idle);
pub static STATUS_BUFFERS: RwLock<LoadingThreadState> = RwLock::new(LoadingThreadState::Idle);
// pub static STATUS_SHADERS: RwLock<LoadingThreadState> = RwLock::new(LoadingThreadState::Idle);

fn update_status(state: &RwLock<LoadingThreadState>, remaining: usize) {
    let status = state.read().clone();
    if remaining == 0 {
        *state.write() = LoadingThreadState::Idle;
    } else {
        match status {
            LoadingThreadState::Idle => {
                *state.write() = LoadingThreadState::Loading {
                    start_time: Instant::now(),
                    remaining,
                }
            }
            LoadingThreadState::Loading { start_time, .. } => {
                *state.write() = LoadingThreadState::Loading {
                    start_time,
                    remaining,
                }
            }
        }
    }
}

pub fn thread_textures(
    dcs: Arc<DeviceContextSwapchain>,
    data: Arc<RwLock<RenderData>>,
) -> mpsc::Sender<TagHash> {
    let (tx, rx) = mpsc::unbounded::<TagHash>();

    std::thread::Builder::new()
        .name("Texture loader".into())
        .spawn(move || {
            while let Ok(hash) = rx.recv() {
                if hash.is_valid() && !data.read().textures.contains_key(&hash) {
                    match Texture::load(&dcs, hash) {
                        Ok(t) => {
                            data.write().textures.insert(hash, t);
                        }
                        Err(e) => error!("Failed to load texture {hash}: {e}"),
                    }
                }

                update_status(&STATUS_TEXTURES, rx.len());
            }

            info!("Texture loading thread exited");
        })
        .unwrap();

    tx
}

pub fn thread_buffers(
    dcs: Arc<DeviceContextSwapchain>,
    render_data: Arc<RwLock<RenderData>>,
) -> mpsc::Sender<TagHash> {
    let (tx, rx) = mpsc::unbounded::<TagHash>();

    std::thread::Builder::new()
        .name("Buffer loader".into())
        .spawn(move || {
            while let Ok(hash) = rx.recv() {
                if hash.is_valid()
                    && !render_data.read().vertex_buffers.contains_key(&hash)
                    && !render_data.read().index_buffers.contains_key(&hash)
                {
                    if let Ok(entry) = package_manager().get_entry(hash) {
                        match (entry.file_type, entry.file_subtype) {
                            // Vertex buffer
                            (32, 4) => match package_manager().read_tag(entry.reference) {
                                Ok(data) => {
                                    let vertex_buffer_header = package_manager()
                                        .read_tag_struct::<VertexBufferHeader>(hash)
                                        .unwrap();

                                    let vertex_buffer = unsafe {
                                        dcs.device
                                            .CreateBuffer(
                                                &D3D11_BUFFER_DESC {
                                                    ByteWidth: data.len() as _,
                                                    Usage: D3D11_USAGE_IMMUTABLE,
                                                    BindFlags: D3D11_BIND_VERTEX_BUFFER,
                                                    ..Default::default()
                                                },
                                                Some(&D3D11_SUBRESOURCE_DATA {
                                                    pSysMem: data.as_ptr() as _,
                                                    ..Default::default()
                                                }),
                                            )
                                            .context("Failed to create vertex buffer")
                                            .unwrap()
                                    };

                                    render_data.write().vertex_buffers.insert(
                                        hash,
                                        (vertex_buffer, vertex_buffer_header.stride as u32),
                                    );
                                }
                                Err(e) => error!("Failed to load vertex buffer {hash}: {e}"),
                            },
                            // Index buffer
                            (32, 6) => match package_manager().read_tag(entry.reference) {
                                Ok(data) => {
                                    let index_buffer_header = package_manager()
                                        .read_tag_struct::<IndexBufferHeader>(hash)
                                        .unwrap();

                                    let index_buffer = unsafe {
                                        dcs.device
                                            .CreateBuffer(
                                                &D3D11_BUFFER_DESC {
                                                    ByteWidth: data.len() as _,
                                                    Usage: D3D11_USAGE_IMMUTABLE,
                                                    BindFlags: D3D11_BIND_INDEX_BUFFER,
                                                    ..Default::default()
                                                },
                                                Some(&D3D11_SUBRESOURCE_DATA {
                                                    pSysMem: data.as_ptr() as _,
                                                    ..Default::default()
                                                }),
                                            )
                                            .context("Failed to create index buffer")
                                            .unwrap()
                                    };

                                    render_data.write().index_buffers.insert(
                                        hash,
                                        (
                                            index_buffer,
                                            if index_buffer_header.is_32bit {
                                                DxgiFormat::R32_UINT
                                            } else {
                                                DxgiFormat::R16_UINT
                                            },
                                        ),
                                    );
                                }
                                Err(e) => error!("Failed to load vertex buffer {hash}: {e}"),
                            },
                            u => panic!("Unsupported mt loader buffer type {u:?}"),
                        }
                    }
                }

                update_status(&STATUS_BUFFERS, rx.len());
            }

            info!("Buffer loading thread exited");
        })
        .unwrap();

    tx
}
