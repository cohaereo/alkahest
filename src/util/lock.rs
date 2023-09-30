use std::ops::{Deref, DerefMut};

use std::sync::atomic::AtomicUsize;
use std::time::Instant;

static LOCK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[allow(unused)]
pub struct LockTracker<T> {
    lock: T,
    id: usize,
    location: Option<String>,
    start_time: Instant,
}

#[allow(unused)]
impl<T> LockTracker<T> {
    pub fn wrap(lock: T, location: Option<String>) -> LockTracker<T> {
        let id = LOCK_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        debug!(
            target: "debug_lock",
            thread =? std::thread::current().id(),
            location =? location,
            id,
            action = "acquired",
        );

        LockTracker {
            lock,
            id,
            location,
            start_time: Instant::now(),
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

impl<T> Drop for LockTracker<T> {
    fn drop(&mut self) {
        debug!(
            target: "debug_lock",
            thread =? std::thread::current().id(),
            location =? self.location,
            self.id,
            time_held_us = self.start_time.elapsed().as_micros(),
            action = "released",
        );
    }
}

impl<T: Deref> Deref for LockTracker<T> {
    type Target = T::Target;
    fn deref(&self) -> &Self::Target {
        self.lock.deref()
    }
}

impl<T: DerefMut> DerefMut for LockTracker<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.lock.deref_mut()
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

#[allow(unused)]
pub(crate) use caller_frame;
use tracing::{Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Filter};

// TODO(cohae): can we integrate this with the C++ tracy lock API?
#[allow(unused)]
pub struct RwLockTracy<T> {
    inner: parking_lot::RwLock<T>,
}

#[allow(unused)]
impl<T> RwLockTracy<T> {
    pub const fn new(value: T) -> RwLockTracy<T> {
        RwLockTracy {
            inner: parking_lot::RwLock::new(value),
        }
    }

    #[inline]
    #[track_caller]
    pub fn read(&self) -> LockTracker<parking_lot::RwLockReadGuard<'_, T>> {
        LockTracker::wrap(self.inner.read(), Some(caller_frame!()))
    }

    #[inline]
    #[track_caller]
    pub fn write(&self) -> LockTracker<parking_lot::RwLockWriteGuard<'_, T>> {
        LockTracker::wrap(self.inner.write(), Some(caller_frame!()))
    }
}

#[cfg(feature = "debug_lock")]
pub type RwLock<T> = RwLockTracy<T>;
#[cfg(not(feature = "debug_lock"))]
pub type RwLock<T> = parking_lot::RwLock<T>;

pub struct FilterDebugLockTarget;
impl<S: Subscriber> Filter<S> for FilterDebugLockTarget {
    fn enabled(&self, meta: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        meta.target() != "debug_lock"
    }
}
