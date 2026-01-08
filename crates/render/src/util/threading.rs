use std::{
    cell::UnsafeCell,
    ops::Deref,
    sync::{Arc, atomic::AtomicUsize},
};

use ahash::HashSet;
use alkahest_core::job::SCHEDULER;
use parking_lot::Mutex;

use crate::{
    Gpu,
    gpu::{command_list::CommandList, state::GpuState},
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CommandListSetId(usize);

struct CommandListSet {
    command_lists: Vec<UnsafeCell<CommandList>>,
}

pub struct CommandListPool {
    sets: Vec<CommandListSet>,
    next_set: AtomicUsize,
    sets_in_use: Mutex<HashSet<usize>>,
}

unsafe impl Send for CommandListPool {}
unsafe impl Sync for CommandListPool {}

impl CommandListPool {
    const NUM_SETS: usize = 12;

    pub fn new(gpu: &Arc<Gpu>) -> Self {
        // let command_lists = (0..SCHEDULER.num_workers())
        //     .map(|_| UnsafeCell::new(gpu.create_command_list()))
        //     .collect::<Vec<_>>();

        let mut sets = Vec::with_capacity(Self::NUM_SETS);
        for _ in 0..Self::NUM_SETS {
            let command_lists = (0..SCHEDULER.num_workers())
                .map(|_| UnsafeCell::new(gpu.create_command_list()))
                .collect::<Vec<_>>();
            sets.push(CommandListSet { command_lists });
        }

        Self {
            sets,
            next_set: AtomicUsize::new(0),
            sets_in_use: Mutex::new(HashSet::default()),
        }
    }

    /// Get a unique index for the current thread.
    fn thread_idx() -> usize {
        static IDX_COUNTER: AtomicUsize = AtomicUsize::new(0);
        thread_local! {
            static THREAD_IDX: usize = IDX_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        THREAD_IDX.with(|idx| *idx)
    }

    #[profiling::function]
    #[allow(clippy::mut_from_ref)]
    pub fn get_command_list_manual(
        &self,
        set: CommandListSetId,
        index: usize,
    ) -> Option<&mut CommandList> {
        let set = &self.sets[set.0 % self.sets.len()];
        let cell = &set.command_lists.get(index)?;
        Some(unsafe { &mut *cell.get() })
    }

    #[profiling::function]
    #[allow(clippy::mut_from_ref)]
    pub fn get_command_list(&self, set: CommandListSetId) -> &mut CommandList {
        let set = &self.sets[set.0 % self.sets.len()];
        let idx = Self::thread_idx() % set.command_lists.len();
        let cell = &set.command_lists[idx];
        unsafe { &mut *cell.get() }
    }

    fn acquire_set(&self) -> CommandListSetId {
        let set_idx = self
            .next_set
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let set = CommandListSetId(set_idx % self.sets.len());
        if self.sets_in_use.lock().contains(&set_idx) {
            panic!(
                "All command list sets are in use! Increase NUM_SETS in CommandListPool \
                 (currently {}).",
                Self::NUM_SETS
            );
        }

        set
    }

    fn release_set(&self, set: CommandListSetId) {
        self.sets_in_use.lock().remove(&set.0);
    }

    // pub fn finalize_set(&self, set: CommandListSetId) {
    //     if self.sets_in_use.lock().contains(&set.0) {
    //         panic!("Command list set {:?} is not in use!", set);
    //     }

    //     let set = &self.sets[set.0 % self.sets.len()];
    //     let combined_cmd = self.gpu.create_command_list();
    //     for cell in set.command_lists.iter() {
    //         let worker_cmd = unsafe { &mut *cell.get() };
    //         combined_cmd.execute_command_list(
    //             &worker_cmd
    //                 .finish_command_list(false)
    //                 .expect("Failed to finalize command list"),
    //             true,
    //         );
    //     }
    //     let finished_cmd = combined_cmd
    //         .finish_command_list(false)
    //         .expect("Failed to finalize combined command list");
    //     *set.finished_command_list.lock() = Some(finished_cmd);
    // }

    /// Copy the given command list's state to all command lists in the pool and begin recording on them.
    /// # Safety
    /// - The caller must ensure that none of the command lists in the pool are being used while this function is called.
    #[profiling::function]
    pub unsafe fn begin(&self, cmd: &mut CommandList) -> CommandListSetId {
        let initial_state = GpuState::backup(cmd);
        let set_id = self.acquire_set();

        let set = &self.sets[set_id.0 % self.sets.len()];
        for cell in set.command_lists.iter() {
            let worker_cmd = unsafe { &mut *cell.get() };
            initial_state.restore(worker_cmd);
            worker_cmd.flush_states();
        }

        set_id
    }

    // #[profiling::function]
    // pub fn get_finalized_command_list(&self, id: CommandListSetId) -> Option<d3d11::CommandList> {
    //     let set = &self.sets[id.0 % self.sets.len()];
    //     let cmd = set.finished_command_list.lock().take();
    //     if cmd.is_some() {
    //         self.release_set(id);
    //     }

    //     cmd
    // }

    /// Execute the finalized command lists onto the given command list.
    #[profiling::function]
    pub fn finish(&self, cmd: &mut CommandList, set: CommandListSetId) {
        {
            let set = &self.sets[set.0 % self.sets.len()];
            for cell in set.command_lists.iter() {
                let worker_cmd = unsafe { &mut *cell.get() };
                let finished_cmd = worker_cmd
                    .finish_command_list(false)
                    .expect("Failed to finalize command list");

                cmd.execute_command_list(&finished_cmd, true);
            }
        }
        self.release_set(set);
    }
}

pub fn parallel_iter<T>(slice: &mut [T], func: impl Fn(&mut T) + Send + Sync)
where
    T: Send + 'static,
{
    struct JobContext<T: Send> {
        chunk: *mut T,
        func: *const dyn Fn(&mut T),
    }

    unsafe impl<T: Send> Send for JobContext<T> {}

    let mut job_handles = Vec::with_capacity(slice.len());
    for item in slice.iter_mut() {
        let context = JobContext {
            chunk: item as *mut T,
            func: &raw const func as *const dyn Fn(&mut T),
        };

        let job_handle = SCHEDULER.job_builder("parallel_iter_chunk").spawn(move || {
            let context = &context;
            let chunk = unsafe { &mut *context.chunk };
            let func = unsafe { &*context.func };
            func(chunk);
        });

        job_handles.push(job_handle);
    }

    let sync_job = SCHEDULER
        .job_builder("parallel_iter_sync")
        .dependencies(job_handles)
        .spawn(|| {});

    sync_job.wait();
}

// pub fn parallel_iter<T>(slice: &mut [T], func: impl Fn(&mut [T]) + Send + Sync)
// where
//     T: Send + 'static,
// {
//     let num_workers = SCHEDULER.num_workers();
//     let chunk_size = slice.len().div_ceil(num_workers);

//     struct JobContext<T: Send> {
//         chunk: *mut [T],
//         func: *const dyn Fn(&mut [T]),
//     }

//     unsafe impl<T: Send> Send for JobContext<T> {}

//     let mut job_handles = Vec::with_capacity(num_workers);
//     for chunk in slice.chunks_mut(chunk_size) {
//         let context = JobContext {
//             chunk: chunk as *mut [T],
//             func: &raw const func as *const dyn Fn(&mut [T]),
//         };

//         let job_handle = SCHEDULER.job_builder("parallel_iter_chunk").spawn(move || {
//             let context = &context;
//             let chunk = unsafe { &mut *context.chunk };
//             let func = unsafe { &*context.func };
//             func(chunk);
//         });

//         job_handles.push(job_handle);
//     }

//     let sync_job = SCHEDULER
//         .job_builder("parallel_iter_sync")
//         .dependencies(job_handles)
//         .spawn(|| {});

//     sync_job.wait();
// }
