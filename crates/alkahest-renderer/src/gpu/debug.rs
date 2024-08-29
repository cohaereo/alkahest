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
    pub fn begin_event<D: AsRef<str>>(&self, name: &str, data: D) -> GpuEventGuard {
        unsafe {
            self.annotation
                .BeginEvent(&HSTRING::from(if data.as_ref().is_empty() {
                    name.to_string()
                } else {
                    format!("{} ({})", name, data.as_ref())
                }))
        };
        GpuEventGuard {
            annotation: self.annotation.clone(),
        }
    }

    pub fn last_device_error(&self) -> Option<String> {
        unsafe {
            self.device
                .GetDeviceRemovedReason()
                .err()
                .map(|e| format!("DirectX error: {e:?}"))
        }
    }
}

#[macro_export]
macro_rules! gpu_event {
    ($gpu:expr, $name:expr) => {
        gpu_event!($gpu, $name, "")
    };
    ($gpu:expr, $name:expr, $data:expr) => {
        let __gpu_eventguard = $gpu.begin_event($name, $data);
        let __puffin_eventguard = if puffin::are_scopes_on() {
            static SCOPE_ID: std::sync::OnceLock<puffin::ScopeId> = std::sync::OnceLock::new();
            let scope_id = SCOPE_ID.get_or_init(|| {
                puffin::ThreadProfiler::call(|tp| {
                    let name = $name.to_string();
                    let id = tp.register_named_scope(
                        name,
                        puffin::clean_function_name(puffin::current_function_name!()),
                        puffin::short_file_name(file!()),
                        line!(),
                    );
                    id
                })
            });
            Some(puffin::ProfilerScope::new(*scope_id, $data))
        } else {
            None
        };
    };
}
