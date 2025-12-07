use std::{any::TypeId, sync::Arc};

use hashbrown::HashMap;
use parking_lot::Mutex;
use tiger_pkg::TagHash;
use uuid::Uuid;

use super::{
    handle::{Handle, UntypedHandle},
    Asset,
};
use crate::{
    asset::{
        index_buffer::{load_index_buffer, IndexBuffer},
        technique::Technique,
        vertex_buffer::{load_vertex_buffer, VertexBuffer},
    },
    Gpu,
};

// Asynchronous asset manager. Allows taking a handle to an ArcShift<Option<T>> (where T: Asset), which will be populated with the asset once it is loaded.
// Works for any asset type that implements the Asset trait.
pub struct AssetManager {
    assets: Mutex<HashMap<TagHash, (TypeId, UntypedHandle)>>,
    request_tx: Mutex<crossbeam::channel::Sender<LoadRequest>>,
    pub(crate) loading_threads: Mutex<Vec<std::thread::JoinHandle<()>>>,
    dummy_handle: UntypedHandle,
}

impl AssetManager {
    pub fn new(gpu: &Arc<Gpu>) -> Self {
        let (request_tx, loading_threads) = asset_loader_threads(gpu.clone());
        Self {
            assets: Mutex::new(HashMap::new()),
            request_tx: Mutex::new(request_tx),
            loading_threads: Mutex::new(loading_threads),
            dummy_handle: UntypedHandle::new(TagHash::NONE),
        }
    }

    pub fn get<T: Asset + 'static>(&self, tag: TagHash) -> Option<Handle<T>> {
        if let Some((ty, handle)) = self.assets.lock().get(&tag) {
            if *ty == TypeId::of::<T>() {
                return Some(unsafe { handle.clone_as_typed_unchecked::<T>() });
            }
        }
        None
    }

    pub fn load<T: Asset + 'static>(&self, tag: TagHash) -> Handle<T> {
        self.try_load(tag)
            .unwrap_or_else(|| unsafe { self.dummy_handle.clone_as_typed_unchecked() })
    }

    /// Get the asset handle for the given tag, or create a new one, and send it to the loader thread.
    /// Returns None if the tag is null
    #[profiling::function]
    pub fn try_load<T: Asset + 'static>(&self, tag: TagHash) -> Option<Handle<T>> {
        if tag.is_none() {
            // TODO: Return a dummy handle instead of None
            return None;
        }

        let mut cache = self.assets.lock();
        if let Some((ty, handle)) = cache.get(&tag) {
            if *ty == TypeId::of::<T>() {
                return Some(unsafe { handle.clone_as_typed_unchecked::<T>() });
            } else {
                error!("AssetManager::try_load: Tag {tag} already loaded with different type");
                return None;
            }
        }

        let handle = UntypedHandle::new(tag);
        cache.insert(tag, (TypeId::of::<T>(), handle.clone()));
        drop(cache);

        self.request_tx
            .lock()
            .send(LoadRequest {
                tag,
                handle: handle.clone(),
                type_id: T::ASSET_TYPE,
            })
            .expect("Failed to send asset load request");

        // SAFETY: The type ID was checked above
        Some(unsafe { handle.clone_as_typed_unchecked() })
    }

    /// Cull assets that are no longer referenced (ref count == 1, since the asset handle itself holds a reference)
    #[profiling::function]
    pub fn remove_unreferenced(&self) {
        self.assets.lock().retain(|t, (_, handle)| {
            if handle.ref_count() == 1 {
                debug!("Culling asset {t}");
            }
            handle.ref_count() > 1
        });
    }

    pub fn count_loading(&self) -> usize {
        self.request_tx.lock().len()
    }

    pub fn shutdown(&self) {
        // Drop the request channel and join all loading threads
        let _ = std::mem::replace(
            &mut *self.request_tx.lock(),
            crossbeam::channel::unbounded().0,
        );
        for thread in self.loading_threads.lock().drain(..) {
            if let Err(e) = thread.join() {
                error!("Failed to join asset loader thread: {:?}", e);
            }
        }
    }
}

struct LoadRequest {
    tag: TagHash,
    handle: UntypedHandle,
    type_id: Uuid,
}

fn asset_loader_threads(
    gpu: Arc<Gpu>,
) -> (
    crossbeam::channel::Sender<LoadRequest>,
    Vec<std::thread::JoinHandle<()>>,
) {
    let (tx, rx) = crossbeam::channel::unbounded();

    let mut threads = Vec::new();
    for i in 0..2 {
        let rx_clone = rx.clone();
        let gpu_clone = gpu.clone();
        threads.push(
            std::thread::Builder::new()
                .name(format!("asset_loader_{i}"))
                .spawn(move || {
                    asset_loader_loop(rx_clone, gpu_clone, i);
                })
                .expect("Failed to spawn asset loader thread"),
        );
    }

    (tx, threads)
}

fn asset_loader_loop(rx: crossbeam::channel::Receiver<LoadRequest>, gpu: Arc<Gpu>, id: usize) {
    use crate::asset::texture::Texture;

    info!("Asset loader thread #{id} started");
    for request in rx {
        match request.type_id {
            Texture::ASSET_TYPE => {
                match Texture::load(&gpu.device, request.tag) {
                    Ok(o) => {
                        request.handle.update(o.into());
                    }
                    Err(e) => {
                        // TODO(cohae): Some more transparent error handling would perhaps be nice? Right now this just leaves the handle without data.
                        error!("Failed to load texture: {:?}", e);
                    }
                }
            }
            VertexBuffer::ASSET_TYPE => match load_vertex_buffer(&gpu, request.tag) {
                Ok(o) => {
                    request.handle.update(o.into());
                }
                Err(e) => {
                    error!("Failed to load vertex buffer: {:?}", e);
                }
            },
            IndexBuffer::ASSET_TYPE => match load_index_buffer(&gpu, request.tag) {
                Ok(o) => {
                    request.handle.update(o.into());
                }
                Err(e) => {
                    error!("Failed to load index buffer: {:?}", e);
                }
            },
            Technique::ASSET_TYPE => match Technique::load(&gpu, request.tag) {
                Ok(o) => {
                    request.handle.update(o.into());
                }
                Err(e) => {
                    error!("Failed to load technique: {:?}", e);
                }
            },
            u => {
                panic!(
                    "asset loader: Unknown asset type for tag {}: {u:?}",
                    request.tag
                );
            }
        }
    }
}
