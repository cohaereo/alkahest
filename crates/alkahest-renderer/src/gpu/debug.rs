use windows::{core::HSTRING, Win32::Graphics::Direct3D11::ID3DUserDefinedAnnotation};

use crate::gpu::GpuContext;

pub struct GpuEventGuard {
    annotation: ID3DUserDefinedAnnotation,
}

impl GpuEventGuard {
    pub fn scoped<F: FnOnce()>(self, f: F) {
        f();
    }
}

impl Drop for GpuEventGuard {
    fn drop(&mut self) {
        unsafe {
            self.annotation.EndEvent();
        }
    }
}

impl GpuContext {
    pub fn begin_event<N: AsRef<str>>(&self, name: N) -> GpuEventGuard {
        unsafe { self.annotation.BeginEvent(&HSTRING::from(name.as_ref())) };
        GpuEventGuard {
            annotation: self.annotation.clone(),
        }
    }
}

#[macro_export]
macro_rules! gpu_event {
    ($gpu:expr, $name:expr) => {
        let __gpu_eventguard = $gpu.begin_event($name);
    };
}
