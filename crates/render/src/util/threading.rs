use std::{cell::UnsafeCell, ops::Deref, sync::Arc};

use crate::{gpu::command_list::CommandList, Gpu};

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

pub struct CommandThreadPool {
    _threads: Vec<std::thread::JoinHandle<()>>,
    sender: crossbeam::channel::Sender<Box<dyn FnOnce(&mut CommandList) + Send + 'static>>,
    receiver: crossbeam::channel::Receiver<d3d11::CommandList>,
}

impl CommandThreadPool {
    pub fn new(thread_count: usize, gpu: &Arc<Gpu>) -> Self {
        let (job_sender, job_receiver) =
            crossbeam::channel::unbounded::<Box<dyn FnOnce(&mut CommandList) + Send + 'static>>();
        let (result_sender, result_receiver) =
            crossbeam::channel::unbounded::<d3d11::CommandList>();

        let mut threads = Vec::with_capacity(thread_count);
        for i in 0..thread_count {
            let gpu = gpu.clone();
            let job_receiver = job_receiver.clone();
            let result_sender = result_sender.clone();
            let handle = std::thread::Builder::new()
                .name(format!("cmd_thread_{i}"))
                .spawn(move || {
                    let mut command_list = gpu.create_command_list();
                    while let Ok(job) = job_receiver.recv() {
                        job(&mut command_list);
                        result_sender
                            .send(command_list.finish_command_list(false).unwrap())
                            .unwrap();
                    }
                })
                .unwrap();
            threads.push(handle);
        }

        Self {
            _threads: threads,
            sender: job_sender,
            receiver: result_receiver,
        }
    }

    pub fn queue_job<F>(&self, job: F)
    where
        F: FnOnce(&mut CommandList) + Send + 'static,
    {
        self.sender.send(Box::new(job)).unwrap();
    }

    pub fn collect_results(&self, count: usize) -> Vec<d3d11::CommandList> {
        let mut results = Vec::with_capacity(count);
        for _ in 0..count {
            if let Ok(result) = self.receiver.recv() {
                results.push(result);
            }
        }
        results
    }
}
