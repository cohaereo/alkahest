use std::{
    any::Any,
    cell::UnsafeCell,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc,
    },
};

use tiger_pkg::TagHash;

use super::Asset;
use crate::tfx::technique::Technique;

struct AssetHolder {
    data: UnsafeCell<Arc<dyn Any + Send + Sync>>,
    loaded: AtomicBool,
    ref_count: AtomicUsize,
}

impl AssetHolder {
    fn new() -> Self {
        Self {
            data: UnsafeCell::new(Arc::new(())),
            loaded: AtomicBool::new(false),
            ref_count: AtomicUsize::new(1),
        }
    }
}

unsafe impl Send for AssetHolder {}
unsafe impl Sync for AssetHolder {}

pub struct UntypedHandle {
    inner: Arc<AssetHolder>,
    pub tag: TagHash,
}

impl Default for UntypedHandle {
    fn default() -> Self {
        Self::new(TagHash::NONE)
    }
}

impl UntypedHandle {
    pub fn new(tag: TagHash) -> Self {
        Self {
            inner: Arc::new(AssetHolder::new()),
            tag,
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.inner.loaded.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// # Safety
    /// The caller must ensure that the asset is of the correct type.
    pub unsafe fn clone_as_typed_unchecked<T: Asset>(&self) -> Handle<T> {
        Handle {
            asset: self.clone(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn update<T: Asset + Send + Sync + 'static>(&self, asset: Box<T>) {
        if self.is_loaded() {
            error!(
                "Attempted to update already loaded asset handle {}",
                self.tag
            );
            return;
        }

        unsafe { *self.inner.data.get() = Arc::<T>::from(asset) };
        self.inner
            .loaded
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn ref_count(&self) -> usize {
        self.inner
            .ref_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Clone for UntypedHandle {
    fn clone(&self) -> Self {
        self.inner
            .ref_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            inner: self.inner.clone(),
            tag: self.tag,
        }
    }
}

impl Drop for UntypedHandle {
    fn drop(&mut self) {
        self.inner
            .ref_count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

pub struct Handle<T: Asset + 'static> {
    asset: UntypedHandle,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Asset + Sync + Send + 'static> Handle<T> {
    /// Returns true if the handle is null or the asset is loaded
    pub fn is_loaded(&self) -> bool {
        self.is_null() || self.asset.is_loaded()
    }

    pub fn is_null(&self) -> bool {
        self.asset.tag.is_none()
    }

    pub fn hash(&self) -> TagHash {
        self.asset.tag
    }

    pub fn get(&self) -> Option<Arc<T>> {
        if !self.is_loaded() {
            return None;
        }

        let data = unsafe { &*self.asset.inner.data.get() };
        // data.downcast_ref().cloned()
        Arc::downcast(Arc::clone(data)).ok()
    }

    // Passes the ref in a closure to avoid cloning the Arc unnecessarily
    pub fn get_ref<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        if !self.is_loaded() {
            return None;
        }

        let data = unsafe { &*self.asset.inner.data.get() };
        let asset = data.downcast_ref::<T>()?;
        Some(f(asset))
    }

    pub fn update(&self, asset: Box<T>) {
        self.asset.update(asset);
    }

    pub fn ref_count(&self) -> usize {
        self.asset.ref_count()
    }
}

impl<T: Asset + 'static> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            asset: self.asset.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

pub fn is_technique_loaded(handle: &Handle<Technique>) -> bool {
    if handle.is_null() {
        return true;
    }

    let Some(technique) = handle.get() else {
        return false;
    };

    technique.is_loaded()
}
