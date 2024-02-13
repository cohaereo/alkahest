use std::{sync::Arc, time::Instant};

use alkahest_data::{
    entity::{IndexBufferHeader, VertexBufferHeader},
    ExtendedHash,
};
use anyhow::Context;
use crossbeam::channel::{self as mpsc, Receiver};
use destiny_pkg::TagHash;
use tiger_parse::PackageManagerExt;
use windows::Win32::Graphics::{
    Direct3D::{WKPDID_D3DDebugObjectName, D3D11_SRV_DIMENSION_BUFFER},
    Direct3D11::{
        D3D11_BIND_INDEX_BUFFER, D3D11_BIND_SHADER_RESOURCE, D3D11_BIND_VERTEX_BUFFER,
        D3D11_BUFFER_DESC, D3D11_BUFFER_SRV, D3D11_BUFFER_SRV_0, D3D11_BUFFER_SRV_1,
        D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA,
        D3D11_USAGE_IMMUTABLE,
    },
    Dxgi::Common::DXGI_FORMAT_R8G8B8A8_UNORM,
};

use super::{DeviceContextSwapchain, RenderData};
use crate::{dxgi::DxgiFormat, packages::package_manager, texture::Texture, util::RwLock};

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

fn spawn_thread_textures(
    dcs: Arc<DeviceContextSwapchain>,
    data: Arc<RwLock<RenderData>>,
    rx: Receiver<ExtendedHash>,
    name: &'static str,
) {
    std::thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            while let Ok(hash) = rx.recv() {
                if hash.is_some() && !data.read().textures.contains_key(&hash.key()) {
                    match Texture::load(&dcs, hash) {
                        Ok(t) => {
                            data.write().textures.insert(hash.key(), t);
                        }
                        Err(e) => error!("Failed to load texture {hash:?}: {e}"),
                    }
                }

                update_status(&STATUS_TEXTURES, rx.len());
            }

            info!("Thread '{}' exited", name);
        })
        .unwrap();
}

pub fn thread_textures(
    dcs: Arc<DeviceContextSwapchain>,
    data: Arc<RwLock<RenderData>>,
) -> mpsc::Sender<ExtendedHash> {
    let (tx, rx) = mpsc::unbounded::<ExtendedHash>();

    spawn_thread_textures(dcs.clone(), data.clone(), rx.clone(), "Texture loader 1");
    spawn_thread_textures(dcs, data, rx, "Texture loader 2");

    tx
}

fn spawn_thread_buffers(
    dcs: Arc<DeviceContextSwapchain>,
    data: Arc<RwLock<RenderData>>,
    rx: Receiver<(TagHash, bool)>,
    name: &'static str,
) {
    std::thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            while let Ok((hash, create_rgba_srv)) = rx.recv() {
                if hash.is_some() {
                    if let Some(entry) = package_manager().get_entry(hash) {
                        match (entry.file_type, entry.file_subtype) {
                            // Vertex buffer
                            (32, 4) => {
                                if data.read().vertex_buffers.contains_key(&hash) {
                                    continue;
                                }

                                match package_manager().read_tag(entry.reference) {
                                    Ok(vertex_data) => {
                                        let vertex_buffer_header = package_manager()
                                            .read_tag_struct::<VertexBufferHeader>(hash)
                                            .unwrap();

                                        let vertex_buffer = unsafe {
                                            dcs.device
                                                .CreateBuffer(
                                                    &D3D11_BUFFER_DESC {
                                                        ByteWidth: vertex_data.len() as _,
                                                        Usage: D3D11_USAGE_IMMUTABLE,
                                                        BindFlags: D3D11_BIND_VERTEX_BUFFER
                                                            | D3D11_BIND_SHADER_RESOURCE,
                                                        ..Default::default()
                                                    },
                                                    Some(&D3D11_SUBRESOURCE_DATA {
                                                        pSysMem: vertex_data.as_ptr() as _,
                                                        ..Default::default()
                                                    }),
                                                )
                                                .context("Failed to create vertex buffer")
                                                .unwrap()
                                        };

                                        let name = format!("VertexBuffer {}\0", hash);
                                        unsafe {
                                            vertex_buffer
                                                .SetPrivateData(
                                                    &WKPDID_D3DDebugObjectName,
                                                    name.len() as u32 - 1,
                                                    Some(name.as_ptr() as _),
                                                )
                                                .ok();
                                        }

                                        let view = if create_rgba_srv {
                                            Some(unsafe {
                                                dcs.device
                                                    .CreateShaderResourceView(
                                                        &vertex_buffer,
                                                        Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                                                            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                                                            ViewDimension:
                                                                D3D11_SRV_DIMENSION_BUFFER,
                                                            Anonymous:
                                                                D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                                                                    Buffer: D3D11_BUFFER_SRV {
                                                                        Anonymous1:
                                                                            D3D11_BUFFER_SRV_0 {
                                                                                ElementOffset: 0,
                                                                            },
                                                                        Anonymous2:
                                                                            D3D11_BUFFER_SRV_1 {
                                                                                NumElements:
                                                                                    vertex_data
                                                                                        .len()
                                                                                        as u32
                                                                                        / vertex_buffer_header
                                                                                            .stride
                                                                                            as u32,
                                                                            },
                                                                    },
                                                                },
                                                        }),
                                                    )
                                                    .context("Failed to create vertex buffer SRV")
                                                    .unwrap()
                                            })
                                        } else {
                                            None
                                        };

                                        data.write().vertex_buffers.insert(
                                            hash,
                                            (
                                                vertex_buffer,
                                                vertex_buffer_header.stride as u32,
                                                view,
                                            ),
                                        );
                                    }
                                    Err(e) => error!("Failed to load vertex buffer {hash}: {e}"),
                                }
                            }
                            // Index buffer
                            (32, 6) => {
                                if data.read().index_buffers.contains_key(&hash) {
                                    continue;
                                }

                                match package_manager().read_tag(entry.reference) {
                                    Ok(index_data) => {
                                        let index_buffer_header = package_manager()
                                            .read_tag_struct::<IndexBufferHeader>(hash)
                                            .unwrap();

                                        let index_buffer = unsafe {
                                            dcs.device
                                                .CreateBuffer(
                                                    &D3D11_BUFFER_DESC {
                                                        ByteWidth: index_data.len() as _,
                                                        Usage: D3D11_USAGE_IMMUTABLE,
                                                        BindFlags: D3D11_BIND_INDEX_BUFFER,
                                                        ..Default::default()
                                                    },
                                                    Some(&D3D11_SUBRESOURCE_DATA {
                                                        pSysMem: index_data.as_ptr() as _,
                                                        ..Default::default()
                                                    }),
                                                )
                                                .context("Failed to create index buffer")
                                                .unwrap()
                                        };

                                        data.write().index_buffers.insert(
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
                                }
                            }
                            u => panic!("Unsupported mt loader buffer type {u:?}"),
                        }
                    }
                }

                update_status(&STATUS_BUFFERS, rx.len());
            }

            info!("Thread '{}' exited", name);
        })
        .unwrap();
}

pub fn thread_buffers(
    dcs: Arc<DeviceContextSwapchain>,
    data: Arc<RwLock<RenderData>>,
) -> mpsc::Sender<(TagHash, bool)> {
    let (tx, rx) = mpsc::unbounded::<(TagHash, bool)>();

    spawn_thread_buffers(dcs.clone(), data.clone(), rx.clone(), "Buffer loader 1");
    // spawn_thread_buffers(dcs, data, rx, "Buffer loader 2");

    tx
}
