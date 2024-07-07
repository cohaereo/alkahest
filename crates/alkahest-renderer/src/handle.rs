use std::{
    fmt::{Debug, Formatter},
    hash::BuildHasherDefault,
    sync::{Arc, Weak},
};

use destiny_pkg::TagHash;
use indexmap::IndexMap;
use rustc_hash::FxHasher;

use crate::{
    gpu::texture::Texture,
    loaders::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer},
    tfx::technique::Technique,
};

#[derive(PartialEq)]
pub enum AssetSource {
    Alkahest = 0,
    Tiger = 1,
}

pub enum AssetIdValue {
    Alkahest(u64),
    Tiger(TagHash),
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct AssetId(u64);

impl AssetId {
    const SOURCE_BITS: u64 = 0b11 << 62;
    const VALUE_MASK: u64 = !Self::SOURCE_BITS;

    pub fn new(source: AssetSource, value: u64) -> Self {
        Self((source as u64) << 62 | (value & Self::VALUE_MASK))
    }

    pub fn new_tiger(taghash: TagHash) -> Self {
        Self::new(AssetSource::Tiger, taghash.0 as u64)
    }

    pub fn new_alkahest(value: u64) -> Self {
        Self::new(AssetSource::Alkahest, value)
    }

    pub fn source(&self) -> AssetSource {
        match (self.0 & Self::SOURCE_BITS) >> 62 {
            0 => AssetSource::Alkahest,
            1 => AssetSource::Tiger,
            _ => unreachable!(),
        }
    }

    pub fn value(&self) -> AssetIdValue {
        let v = self.0 & Self::VALUE_MASK;
        match self.source() {
            AssetSource::Alkahest => AssetIdValue::Alkahest(v),
            AssetSource::Tiger => AssetIdValue::Tiger(TagHash(v as u32)),
        }
    }

    pub fn tiger_taghash(&self) -> Option<TagHash> {
        match self.value() {
            AssetIdValue::Tiger(taghash) => Some(taghash),
            _ => None,
        }
    }

    pub fn alkahest_id(&self) -> Option<u64> {
        match self.value() {
            AssetIdValue::Alkahest(id) => Some(id),
            _ => None,
        }
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::new_alkahest(0)
    }
}

impl From<TagHash> for AssetId {
    fn from(taghash: TagHash) -> Self {
        Self::new_tiger(taghash)
    }
}

impl Debug for AssetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.value() {
            AssetIdValue::Alkahest(id) => write!(f, "Alkahest({})", id),
            AssetIdValue::Tiger(taghash) => write!(f, "Tiger({})", taghash),
        }
    }
}

// TODO(cohae): Consider merging weak and strong handles as an enum (see Bevy's AssetHandle)
pub struct Handle<T: Asset> {
    refcount: Arc<()>,
    id: AssetId,
    _phantom: std::marker::PhantomData<T>,
}

// pub struct WeakHandle<T: Asset> {
//     refcount: Weak<()>,
//     id: AssetId,
//     _phantom: std::marker::PhantomData<T>,
// }

pub type RawHandle = Handle<()>;

impl<T: Asset> Handle<T> {
    pub fn none() -> Self {
        Self {
            refcount: Arc::new(()),
            id: AssetId::new_alkahest(0),
            _phantom: std::marker::PhantomData,
        }
    }

    // pub fn downgrade(&self) -> WeakHandle<T> {
    //     WeakHandle {
    //         refcount: Arc::downgrade(&self.refcount),
    //         id: self.id,
    //         _phantom: std::marker::PhantomData,
    //     }
    // }

    pub fn is_none(&self) -> bool {
        self.id == AssetId::new_alkahest(0)
    }

    pub fn from_raw(u: Handle<()>) -> Self {
        Self {
            refcount: Arc::clone(&u.refcount),
            id: u.id,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn to_raw(self) -> Handle<()> {
        Handle {
            refcount: Arc::clone(&self.refcount),
            id: self.id,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }
}

impl<T: Asset> Default for Handle<T> {
    fn default() -> Self {
        Self::none()
    }
}

impl<T: Asset> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            refcount: Arc::clone(&self.refcount),
            ..*self
        }
    }
}
//
// impl<T: Asset> WeakHandle<T> {
//     pub fn upgrade(&self) -> Option<Handle<T>> {
//         self.refcount.upgrade().map(|r| Handle {
//             refcount: r,
//             id: self.id,
//             _phantom: std::marker::PhantomData,
//         })
//     }
// }
//
// impl<T: Asset> Clone for WeakHandle<T> {
//     fn clone(&self) -> Self {
//         Self {
//             refcount: self.refcount.clone(),
//             id: self.id,
//             _phantom: std::marker::PhantomData,
//         }
//     }
// }

pub trait Asset: Sized {}

impl Asset for () {}
impl Asset for Texture {}
impl Asset for Technique {}
impl Asset for VertexBuffer {}
impl Asset for IndexBuffer {}

struct AssetStorage<T: Asset> {
    refcount: Weak<()>,
    asset: Option<Arc<T>>,
}

type FastHasher = BuildHasherDefault<FxHasher>;

pub struct AssetRegistry<T: Asset> {
    handle_map: IndexMap<AssetId, AssetStorage<T>, FastHasher>,
    next_id: usize,
    disabled: bool,
}

impl<T: Asset + 'static> AssetRegistry<T> {
    pub fn new(enabled: bool) -> Self {
        Self {
            handle_map: IndexMap::with_hasher(FastHasher::default()),
            next_id: 0,
            disabled: !enabled,
        }
    }

    // pub fn reserve_handle(&mut self) -> Handle<T> {
    //     let id = self.next_id;
    //     self.next_id += 1;
    //     Handle {
    //         ref_count: Arc::new(()),
    //         id: AssetId::new_alkahest(id as u64),
    //         _phantom: std::marker::PhantomData,
    //     }
    // }

    /// Reserve handle or return the existing handle if it already exists
    pub fn get_handle_tiger(&mut self, taghash: TagHash) -> Handle<T> {
        if taghash.is_none() || self.disabled {
            return Handle::none();
        }
        let id = AssetId::new_tiger(taghash);

        if let Some(h) = self.handle_map.get(&id).and_then(|h| h.refcount.upgrade()) {
            Handle {
                refcount: h,
                id,
                _phantom: std::marker::PhantomData,
            }
        } else {
            let h = Handle {
                refcount: Arc::new(()),
                id,
                _phantom: std::marker::PhantomData,
            };

            self.handle_map.insert(
                h.id,
                AssetStorage {
                    refcount: Arc::downgrade(&h.refcount),
                    asset: None,
                },
            );

            h
        }
    }

    pub fn get_existing_handle_tiger(&self, taghash: TagHash) -> Option<Handle<T>> {
        if taghash.is_none() || self.disabled {
            return None;
        }
        let id = AssetId::new_tiger(taghash);

        self.handle_map
            .get(&id)
            .and_then(|h| h.refcount.upgrade())
            .map(|h| Handle {
                refcount: h,
                id,
                _phantom: std::marker::PhantomData,
            })
    }

    pub fn exists(&self, asset_id: AssetId) -> bool {
        self.disabled || self.handle_map.contains_key(&asset_id)
    }

    /// Overwrite the asset associated with the handle
    pub fn overwrite(&mut self, handle: RawHandle, asset: T) {
        if self.disabled {
            return;
        }
        let id = handle.id;
        if let Some(storage) = self.handle_map.get_mut(&id) {
            storage.asset.insert(Arc::new(asset));
        } else {
            error!("Tried to overwrite non-existent asset {id:?}")
        }
    }

    pub fn insert(&mut self, asset: T) -> Handle<T> {
        if self.disabled {
            return Handle::none();
        }
        let id = self.next_id;
        self.next_id += 1;
        let handle = Handle {
            refcount: Arc::new(()),
            id: AssetId::new_alkahest(id as u64),
            _phantom: std::marker::PhantomData,
        };

        self.handle_map.insert(
            handle.id,
            AssetStorage {
                refcount: Arc::downgrade(&handle.refcount),
                asset: Some(Arc::new(asset)),
            },
        );
        handle
    }

    pub fn get(&self, handle: &Handle<T>) -> Option<&T> {
        if handle.is_none() || self.disabled {
            return None;
        }

        self.handle_map
            .get(&handle.id)
            .and_then(|storage| storage.asset.as_ref().map(|v| v.as_ref()))
    }

    pub fn get_shared(&self, handle: &Handle<T>) -> Option<Arc<T>> {
        if handle.is_none() || self.disabled {
            return None;
        }

        self.handle_map
            .get(&handle.id)
            .and_then(|storage| storage.asset.clone())
    }

    pub fn remove_all_dead(&mut self) -> usize {
        let mut removed = 0;
        for idx in (0..self.handle_map.len()).rev() {
            let element = self.handle_map.get_index(idx).unwrap().1;
            if element.refcount.strong_count() == 0 {
                _ = self.handle_map.swap_remove_index(idx).unwrap();
                removed += 1;
            }
        }

        removed
    }
}
