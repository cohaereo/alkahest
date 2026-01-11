use std::{
    any::TypeId,
    sync::{Arc, atomic::AtomicUsize},
};

use alkahest_core::job::{SCHEDULER, potassium::Priority};
use alkahest_data::tag::WideHash;
use hashbrown::HashMap;
use parking_lot::Mutex;
use tiger_pkg::TagHash;
use uuid::Uuid;

use super::{
    Asset,
    handle::{Handle, UntypedHandle},
};
use crate::{
    Gpu,
    asset::{
        index_buffer::{IndexBuffer, load_index_buffer},
        technique::Technique,
        texture::Texture,
        vertex_buffer::{VertexBuffer, load_vertex_buffer},
    },
};

// Asynchronous asset manager. Allows taking a handle to an ArcShift<Option<T>> (where T: Asset), which will be populated with the asset once it is loaded.
// Works for any asset type that implements the Asset trait.
pub struct AssetManager {
    gpu: Arc<Gpu>,
    assets: Mutex<HashMap<TagHash, (TypeId, UntypedHandle)>>,
    num_loading: Arc<AtomicUsize>,
    // request_tx: Mutex<crossbeam::channel::Sender<LoadRequest>>,
    // pub(crate) loading_threads: Mutex<Vec<std::thread::JoinHandle<()>>>,
    dummy_handle: UntypedHandle,
}

impl AssetManager {
    pub fn new(gpu: &Arc<Gpu>) -> Self {
        Self {
            gpu: gpu.clone(),
            assets: Mutex::new(HashMap::new()),
            num_loading: Arc::new(AtomicUsize::new(0)),
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

    pub fn load<T: Asset + 'static>(&self, tag: impl Into<WideHash>) -> Handle<T> {
        self.try_load(tag)
            .unwrap_or_else(|| unsafe { self.dummy_handle.clone_as_typed_unchecked() })
    }

    /// Get the asset handle for the given tag, or create a new one, and send it to the loader thread.
    /// Returns None if the tag is null
    #[profiling::function]
    pub fn try_load<T: Asset + 'static>(&self, tag: impl Into<WideHash>) -> Option<Handle<T>> {
        let tag = tag.into().hash32();
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

        self.num_loading
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let request = LoadRequest {
            tag,
            handle: handle.clone(),
            type_id: T::ASSET_TYPE,
        };
        let gpu = self.gpu.clone();
        let num_loaded = self.num_loading.clone();
        SCHEDULER
            .job_builder("load_asset")
            .priority(Priority::Low)
            .spawn(move || {
                load_asset(request, &gpu, &num_loaded);
            });

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
        self.num_loading.load(std::sync::atomic::Ordering::SeqCst)
    }
}

struct LoadRequest {
    tag: TagHash,
    handle: UntypedHandle,
    type_id: Uuid,
}

fn load_asset(request: LoadRequest, gpu: &Arc<Gpu>, num_loaded: &Arc<AtomicUsize>) {
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
        VertexBuffer::ASSET_TYPE => match load_vertex_buffer(gpu, request.tag) {
            Ok(o) => {
                request.handle.update(o.into());
            }
            Err(e) => {
                error!("Failed to load vertex buffer: {:?}", e);
            }
        },
        IndexBuffer::ASSET_TYPE => match load_index_buffer(gpu, request.tag) {
            Ok(o) => {
                request.handle.update(o.into());
            }
            Err(e) => {
                error!("Failed to load index buffer: {:?}", e);
            }
        },
        Technique::ASSET_TYPE => match Technique::load(gpu, request.tag) {
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

    num_loaded.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
}
