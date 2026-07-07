pub struct Task<T: Sized + Send + 'static> {
    join_handle: Option<std::thread::JoinHandle<T>>,
}

impl<T: Sized + Send> Task<T> {
    pub fn new<F>(name: String, f: F) -> Self
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let join_handle = std::thread::Builder::new()
            .name(name)
            .spawn(f)
            .expect("Failed to spawn task thread");

        Self {
            join_handle: Some(join_handle),
        }
    }

    pub fn is_pending(&self) -> bool {
        self.join_handle.as_ref().is_some_and(|s| !s.is_finished())
    }

    pub fn get(&mut self) -> Option<std::thread::Result<T>> {
        if self.join_handle.as_ref()?.is_finished() {
            Some(self.join_handle.take().unwrap().join())
        } else {
            None
        }
    }
}

impl<T: Sized + Send> Default for Task<T> {
    fn default() -> Self {
        Self { join_handle: None }
    }
}
