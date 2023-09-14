use std::ops::{Deref, DerefMut};

#[cfg(feature = "debug_lock")]
use std::sync::atomic::AtomicUsize;

#[cfg(feature = "debug_lock")]
static LOCK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct LockTracker<T> {
    lock: T,
    #[cfg(feature = "debug_lock")]
    id: usize,
}

impl<T> LockTracker<T> {
    pub fn wrap(lock: T) -> LockTracker<T> {
        #[cfg(feature = "debug_lock")]
        let id = LOCK_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        LockTracker {
            lock,

            #[cfg(feature = "debug_lock")]
            id,
        }
    }

    #[cfg(feature = "debug_lock")]
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

#[allow(unused)]
macro_rules! caller_frame {
    () => {{
        let caller_location = std::panic::Location::caller();
        let caller_file = caller_location.file();
        let caller_line = caller_location.line();
        format!("{caller_file}:{caller_line}")
    }};
}

pub(crate) use caller_frame;
