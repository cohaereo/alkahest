use crossbeam::channel::{Receiver, Sender};
use destiny_pkg::TagHash;
use rustc_hash::FxHashSet;
use strum::AsRefStr;

use crate::{
    gpu::{texture::Texture, SharedGpuContext},
    handle::{AssetId, AssetIdValue, AssetRegistry, Handle, RawHandle},
    loaders::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer},
    tfx::technique::Technique,
    util::{d3d::ErrorExt, packages::TagHashExt},
};

pub mod index_buffer;
pub mod map;
pub mod technique;
pub mod texture;
pub mod vertex_buffer;

pub struct AssetManager {
    gctx: SharedGpuContext,
    disabled: bool,

    pub textures: AssetRegistry<Texture>,
    pub techniques: AssetRegistry<Technique>,

    pub vertex_buffers: AssetRegistry<VertexBuffer>,
    pub index_buffers: AssetRegistry<IndexBuffer>,

    request_tx: Sender<LoadRequest>,
    asset_rx: Receiver<LoadedAsset>,
    workers: Vec<std::thread::JoinHandle<()>>,

    pending_requests: FxHashSet<AssetId>,
}

impl AssetManager {
    pub fn new(gctx: SharedGpuContext) -> Self {
        let (request_tx, request_rx) = crossbeam::channel::unbounded();
        let (asset_tx, asset_rx) = crossbeam::channel::unbounded();

        let workers = spawn_load_workers(gctx.clone(), 4, request_rx, asset_tx);

        Self {
            gctx,
            disabled: false,
            textures: AssetRegistry::new(true),
            techniques: AssetRegistry::new(true),
            vertex_buffers: AssetRegistry::new(true),
            index_buffers: AssetRegistry::new(true),
            request_tx,
            asset_rx,
            workers,
            pending_requests: FxHashSet::default(),
        }
    }

    pub fn new_disabled(gctx: SharedGpuContext) -> Self {
        let (request_tx, _request_rx) = crossbeam::channel::unbounded();
        let (_asset_tx, asset_rx) = crossbeam::channel::unbounded();

        Self {
            gctx,
            disabled: true,
            textures: AssetRegistry::new(false),
            techniques: AssetRegistry::new(false),
            vertex_buffers: AssetRegistry::new(false),
            index_buffers: AssetRegistry::new(false),
            request_tx,
            asset_rx,
            workers: vec![],
            pending_requests: FxHashSet::default(),
        }
    }

    // TODO(cohae): Can we do something about the boilerplate?
    pub fn get_or_load_texture(&mut self, hash: TagHash) -> Handle<Texture> {
        if hash.is_none() || self.disabled {
            return Handle::none();
        }

        if !self.textures.exists(AssetId::new_tiger(hash)) {
            let h = self.textures.get_handle_tiger(hash);
            self.pending_requests.insert(h.id());
            self.request_tx
                .send(LoadRequest::Texture(h.clone().to_raw()))
                .unwrap();
            h
        } else {
            self.textures.get_handle_tiger(hash)
        }
    }

    #[track_caller]
    pub fn get_or_load_technique(&mut self, hash: TagHash) -> Handle<Technique> {
        if hash.is_none() || self.disabled {
            return Handle::none();
        }

        if !hash.is_valid_pkg() {
            let caller = std::panic::Location::caller();
            error!(
                "Invalid technique hash {hash} requested at {}:{}",
                caller.file(),
                caller.line()
            );
            return Handle::none();
        }

        if !self.techniques.exists(AssetId::new_tiger(hash)) {
            let h = self.techniques.get_handle_tiger(hash);
            self.pending_requests.insert(h.id());
            self.request_tx
                .send(LoadRequest::Technique(h.clone().to_raw()))
                .unwrap();
            h
        } else {
            self.techniques.get_handle_tiger(hash)
        }
    }

    pub fn get_or_load_vertex_buffer(&mut self, hash: TagHash) -> Handle<VertexBuffer> {
        if hash.is_none() || self.disabled {
            return Handle::none();
        }

        if !self.vertex_buffers.exists(AssetId::new_tiger(hash)) {
            let h = self.vertex_buffers.get_handle_tiger(hash);
            self.pending_requests.insert(h.id());
            self.request_tx
                .send(LoadRequest::VertexBuffer(h.clone().to_raw()))
                .unwrap();
            h
        } else {
            self.vertex_buffers.get_handle_tiger(hash)
        }
    }

    pub fn get_or_load_index_buffer(&mut self, hash: TagHash) -> Handle<IndexBuffer> {
        if hash.is_none() || self.disabled {
            return Handle::none();
        }

        if !self.index_buffers.exists(AssetId::new_tiger(hash)) {
            let h = self.index_buffers.get_handle_tiger(hash);
            self.pending_requests.insert(h.id());
            self.request_tx
                .send(LoadRequest::IndexBuffer(h.clone().to_raw()))
                .unwrap();
            h
        } else {
            self.index_buffers.get_handle_tiger(hash)
        }
    }

    pub fn poll(&mut self) {
        if self.disabled {
            return;
        }

        profiling::scope!("AssetManager::poll");
        let mut budget = self.asset_rx.len();
        if budget != 0 {
            debug!("Polling asset manager ({} assets to process)", budget);
        }

        let mut total_removed = 0;
        total_removed += self.textures.remove_all_dead();
        total_removed += self.techniques.remove_all_dead();
        total_removed += self.vertex_buffers.remove_all_dead();
        total_removed += self.index_buffers.remove_all_dead();

        if total_removed > 0 {
            trace!("Removed {total_removed} dead assets");
        }

        while budget > 0 {
            match self.asset_rx.try_recv() {
                Ok(asset) => {
                    debug!(
                        "Received loaded asset handle {:?} of type {}",
                        asset.handle().id(),
                        asset.as_ref(),
                    );

                    self.pending_requests.remove(&asset.handle().id());

                    match asset {
                        LoadedAsset::Texture(h, t) => match t {
                            Ok(t) => {
                                self.textures.overwrite(h, t);
                            }
                            Err(e) => {
                                error!(
                                    "Failed to load texture {:?}: {:?}",
                                    h.id(),
                                    e.with_d3d_error(&self.gctx)
                                );
                            }
                        },
                        LoadedAsset::Technique(h, t) => match t {
                            Ok(mut t) => {
                                for (shader, stage) in t.all_stages_mut() {
                                    if let Some(stage) = stage {
                                        for assignment in shader.textures.iter() {
                                            let texture = self
                                                .get_or_load_texture(assignment.texture);
                                            stage.textures.push((assignment.slot, texture));
                                        }
                                    }
                                }

                                self.techniques.overwrite(h, t);
                            }
                            Err(e) => {
                                error!(
                                    "Failed to load technique {:?}: {:?}",
                                    h.id(),
                                    e.with_d3d_error(&self.gctx)
                                );
                            }
                        },
                        LoadedAsset::VertexBuffer(h, vb) => match vb {
                            Ok(vb) => {
                                self.vertex_buffers.overwrite(h, vb);
                            }
                            Err(e) => {
                                error!(
                                    "Failed to load vertex buffer {:?}: {:?}",
                                    h.id(),
                                    e.with_d3d_error(&self.gctx)
                                );
                            }
                        },
                        LoadedAsset::IndexBuffer(h, ib) => match ib {
                            Ok(ib) => {
                                self.index_buffers.overwrite(h, ib);
                            }
                            Err(e) => {
                                error!(
                                    "Failed to load index buffer {:?}: {:?}",
                                    h.id(),
                                    e.with_d3d_error(&self.gctx)
                                );
                            }
                        },
                    }
                }
                Err(_) => break,
            }

            budget -= 1;
        }
    }

    /// Blocks until all pending requests have been processed.
    pub fn block_until_idle(&mut self) {
        if self.disabled {
            return;
        }

        profiling::scope!("AssetManager::block_until_idle");
        while !self.pending_requests.is_empty() {
            self.poll();
        }
    }

    pub fn is_idle(&self) -> bool {
        self.pending_requests.is_empty()
    }

    pub fn remaining_requests(&self) -> usize {
        self.pending_requests.len()
    }
}

#[derive(AsRefStr)]
pub enum LoadedAsset {
    Texture(RawHandle, anyhow::Result<Texture>),
    Technique(RawHandle, anyhow::Result<Technique>),
    VertexBuffer(RawHandle, anyhow::Result<VertexBuffer>),
    IndexBuffer(RawHandle, anyhow::Result<IndexBuffer>),
}

impl LoadedAsset {
    pub fn handle(&self) -> &RawHandle {
        match self {
            Self::Texture(h, _) => h,
            Self::Technique(h, _) => h,
            Self::VertexBuffer(h, _) => h,
            Self::IndexBuffer(h, _) => h,
        }
    }
}

#[derive(AsRefStr)]
pub enum LoadRequest {
    Texture(RawHandle),
    Technique(RawHandle),
    VertexBuffer(RawHandle),
    IndexBuffer(RawHandle),
}

impl LoadRequest {
    pub fn handle(&self) -> &RawHandle {
        match self {
            Self::Texture(h) => h,
            Self::Technique(h) => h,
            Self::VertexBuffer(h) => h,
            Self::IndexBuffer(h) => h,
        }
    }
}

fn load_worker_thread(
    gctx: SharedGpuContext,
    rx_request: Receiver<LoadRequest>,
    tx: Sender<LoadedAsset>,
) -> anyhow::Result<()> {
    profiling::register_thread!();
    loop {
        match rx_request.recv() {
            Ok(request) => {
                profiling::scope!(
                    "load_worker_thread::handle_request",
                    &format!("{} {:?}", request.as_ref(), request.handle().id())
                );
                match request {
                    LoadRequest::Texture(h) => match h.id().value() {
                        AssetIdValue::Alkahest(_e) => {
                            todo!(
                                "Alkahest custom texture loading unimplemented (texture handle \
                                 {:?})",
                                h.id()
                            );
                        }
                        AssetIdValue::Tiger(hash) => {
                            let t = texture::load_texture(&gctx, hash);
                            tx.send(LoadedAsset::Texture(h, t))?;
                        }
                    },
                    LoadRequest::Technique(h) => match h.id().value() {
                        AssetIdValue::Alkahest(_e) => {
                            error!(
                                "Alkahest custom technique loading is not supported (technique \
                                 handle {:?})",
                                h.id()
                            );
                        }
                        AssetIdValue::Tiger(hash) => {
                            let t = technique::load_technique(gctx.clone(), hash);
                            tx.send(LoadedAsset::Technique(h, t))?;
                        }
                    },
                    LoadRequest::VertexBuffer(h) => match h.id().value() {
                        AssetIdValue::Alkahest(_e) => {
                            todo!(
                                "Alkahest vertex buffer loading unimplemented (vertex buffer \
                                 handle {:?})",
                                h.id()
                            );
                        }
                        AssetIdValue::Tiger(hash) => {
                            let vb = vertex_buffer::load_vertex_buffer(&gctx, hash);
                            tx.send(LoadedAsset::VertexBuffer(h, vb))?;
                        }
                    },
                    LoadRequest::IndexBuffer(h) => match h.id().value() {
                        AssetIdValue::Alkahest(_e) => {
                            todo!(
                                "Alkahest index buffer loading unimplemented (index buffer handle \
                                 {:?})",
                                h.id()
                            );
                        }
                        AssetIdValue::Tiger(hash) => {
                            let ib = index_buffer::load_index_buffer(&gctx, hash);
                            tx.send(LoadedAsset::IndexBuffer(h, ib))?;
                        }
                    },
                }
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }
}

pub fn spawn_load_workers(
    gctx: SharedGpuContext,
    num_workers: usize,
    rx_request: Receiver<LoadRequest>,
    tx: Sender<LoadedAsset>,
) -> Vec<std::thread::JoinHandle<()>> {
    (0..num_workers)
        .map(|i| {
            let gctx = gctx.clone();
            let rx_request = rx_request.clone();
            let tx = tx.clone();

            std::thread::Builder::new()
                .name(format!("alkahest-loader-{i}"))
                .spawn(move || match load_worker_thread(gctx, rx_request, tx) {
                    Ok(_) => {}
                    Err(e) => {
                        debug!("Loader thread exited: {:?}", e);
                    }
                })
                .unwrap()
        })
        .collect()
}
