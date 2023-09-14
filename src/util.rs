use std::{
    ops::{Deref, DerefMut},
    sync::atomic::AtomicUsize,
};

static LOCK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct LockTracker<T> {
    lock: T,
    id: usize,
}

impl<T> LockTracker<T> {
    pub fn wrap(lock: T) -> LockTracker<T> {
        let id = LOCK_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        LockTracker { lock, id }
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

#[cfg(feature = "debug_lock")]
impl<T> Drop for LockTracker<T> {
    fn drop(&mut self) {
        debug!(
            "Thread {:?} is dropping lock #{}",
            std::thread::current().id(),
            self.id
        );
    }
}

impl<T> Deref for LockTracker<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.lock
    }
}

impl<T> DerefMut for LockTracker<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lock
    }
}
