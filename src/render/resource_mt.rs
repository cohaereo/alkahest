use crossbeam::channel as mpsc;
use destiny_pkg::TagHash;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Instant;

use crate::texture::Texture;

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
// pub static STATUS_BUFFERS: RwLock<LoadingThreadState> = RwLock::new(LoadingThreadState::Idle);
// pub static STATUS_SHADERS: RwLock<LoadingThreadState> = RwLock::new(LoadingThreadState::Idle);

pub fn thread_textures(
    dcs: Arc<DeviceContextSwapchain>,
    data: Arc<RwLock<RenderData>>,
) -> mpsc::Sender<TagHash> {
    let (tx, rx) = mpsc::unbounded::<TagHash>();

    std::thread::Builder::new()
        .name("Texture loader".into())
        .spawn(move || {
            while let Ok(hash) = rx.recv() {
                if hash.is_valid() && !data.read().textures.contains_key(&hash.0) {
                    match Texture::load(&dcs, hash) {
                        Ok(t) => {
                            data.write().textures.insert(hash.0, t);
                        }
                        Err(e) => error!("Failed to load texture {hash}: {e}"),
                    }
                }

                let status = STATUS_TEXTURES.read().clone();
                if rx.is_empty() {
                    *STATUS_TEXTURES.write() = LoadingThreadState::Idle;
                } else {
                    match status {
                        LoadingThreadState::Idle => {
                            *STATUS_TEXTURES.write() = LoadingThreadState::Loading {
                                start_time: Instant::now(),
                                remaining: rx.len(),
                            }
                        }
                        LoadingThreadState::Loading { start_time, .. } => {
                            *STATUS_TEXTURES.write() = LoadingThreadState::Loading {
                                start_time,
                                remaining: rx.len(),
                            }
                        }
                    }
                }
            }

            info!("Texture loading thread exited");
        })
        .unwrap();

    tx
}
