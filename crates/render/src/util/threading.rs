use std::{
    cell::UnsafeCell,
    ops::Deref,
    sync::{atomic::AtomicUsize, Arc},
};

use alkahest_core::job::SCHEDULER;
use parking_lot::Mutex;

use crate::{
    gpu::{command_list::CommandList, state::GpuState},
    Gpu,
};

// cohae: This is a kinda stinky way to prevent stuff being mutated on jobs that didn't create the value (ie. the renderer may only be mutated on the main thread).
// It should technically be unsafe, since you can get multiple mutable references to the same value, and the value can be mutated while it's being read, so its only slightly safer than an UnsafeCell
// But I trust myself :) (i think...)
pub struct ThreadMutCell<T> {
    inner: UnsafeCell<T>,
    /// The ID of the thread that created this cell. This thread is the only one that can mutate the inner value.
    thread: std::thread::ThreadId,
}

impl<T> ThreadMutCell<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: UnsafeCell::new(inner),
            thread: std::thread::current().id(),
        }
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.inner.get() }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn get_mut(&self) -> &mut T {
        if std::thread::current().id() != self.thread {
            panic!(
                "Attempted to get mutable reference to ThreadMutCell from a different thread than \
                 the one that created it"
            );
        }
        unsafe { &mut *self.inner.get() }
    }
}

unsafe impl<T: Send> Send for ThreadMutCell<T> {}
unsafe impl<T: Sync> Sync for ThreadMutCell<T> {}

impl<T> Deref for ThreadMutCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

#[derive(Clone)]
pub struct CommandListPool {
    command_lists: Arc<Vec<UnsafeCell<CommandList>>>,
}

unsafe impl Send for CommandListPool {}

impl CommandListPool {
    pub fn new(gpu: &Arc<Gpu>) -> Self {
        let command_lists = (0..SCHEDULER.num_workers())
            .map(|_| UnsafeCell::new(gpu.create_command_list()))
            .collect::<Vec<_>>();

        Self {
            command_lists: Arc::new(command_lists),
        }
    }

    fn thread_idx() -> usize {
        static IDX_COUNTER: AtomicUsize = AtomicUsize::new(0);
        thread_local! {
            static THREAD_IDX: usize = IDX_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        THREAD_IDX.with(|idx| *idx)
    }

    #[profiling::function]
    #[allow(clippy::mut_from_ref)]
    pub fn get_command_list(&self) -> &mut CommandList {
        let idx = Self::thread_idx() % self.command_lists.len();
        let cell = &self.command_lists[idx];
        unsafe { &mut *cell.get() }
    }

    /// Copy the given command list's state to all command lists in the pool and begin recording on them.
    /// # Safety
    /// - The caller must ensure that none of the command lists in the pool are being used while this function is called.
    #[profiling::function]
    pub unsafe fn begin(&self, cmd: &mut CommandList) {
        let initial_state = GpuState::backup(cmd);
        for cell in self.command_lists.iter() {
            let worker_cmd = unsafe { &mut *cell.get() };
            initial_state.restore(worker_cmd);
        }
    }

    /// Finish all command lists in the pool and execute them on the given command list.
    /// # Safety
    /// - The caller must ensure that none of the command lists in the pool are being used while this function is called.
    #[profiling::function]
    pub unsafe fn finish(&self, cmd: &mut CommandList) {
        for cell in self.command_lists.iter() {
            let worker_cmd = unsafe { &mut *cell.get() };
            cmd.execute_command_list(
                &worker_cmd
                    .finish_command_list(false)
                    .expect("Failed to finish command list"),
                true,
            );
        }
    }
}
