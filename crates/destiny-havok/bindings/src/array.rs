#[repr(C)]
pub struct CArray<T> {
    pub data: *mut T,
    pub len: usize,
}

impl<T> CArray<T> {
    pub fn new(data: Box<[T]>) -> Self {
        Self {
            len: data.len(),
            data: Box::into_raw(data) as *mut _,
        }
    }
}

impl<T> Drop for CArray<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.data);
        };
    }
}
