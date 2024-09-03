use std::time::Duration;

use windows::{
    core::{Interface, Param, HRESULT, HSTRING},
    Win32::{
        Foundation::{S_FALSE, S_OK},
        Graphics::Direct3D11::{
            ID3D11DeviceContext, ID3D11Query, ID3DUserDefinedAnnotation, D3D11_QUERY,
            D3D11_QUERY_DATA_TIMESTAMP_DISJOINT, D3D11_QUERY_DESC, D3D11_QUERY_TIMESTAMP,
            D3D11_QUERY_TIMESTAMP_DISJOINT,
        },
    },
};

use crate::{gpu::GpuContext, util::d3d::try_out_ptr};

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

pub struct GpuProfilingGuard {
    disjoint: ID3D11Query,
    end: ID3D11Query,
    context: ID3D11DeviceContext,
}

impl Drop for GpuProfilingGuard {
    fn drop(&mut self) {
        unsafe {
            self.context.End(&self.end);
            self.context.End(&self.disjoint);
        }
    }
}

pub struct PendingGpuTimestampRange {
    label: String,
    disjoint: ID3D11Query,
    start: ID3D11Query,
    end: ID3D11Query,
}

impl PendingGpuTimestampRange {
    /// Tries to resolve the queries to a [`GpuTimestampRange`]. Returns None if the data is not yet available
    pub fn resolve(&self, gpu: &GpuContext) -> Option<GpuTimestampRange> {
        unsafe {
            let disjoint: D3D11_QUERY_DATA_TIMESTAMP_DISJOINT =
                gpu.get_query_data(&self.disjoint).unwrap()?;

            let start: u64 = gpu.get_query_data(&self.start).unwrap()?;
            let end: u64 = gpu.get_query_data(&self.end).unwrap()?;

            Some(GpuTimestampRange {
                label: self.label.clone(),
                disjoint: disjoint.Disjoint.into(),
                frequency: disjoint.Frequency,
                start,
                end,
            })
        }
    }

    /// Tries to resolve the queries to a [`GpuTimestampRange`]
    pub fn resolve_blocking(&self, gpu: &GpuContext) -> GpuTimestampRange {
        loop {
            if let Some(timestamp) = self.resolve(gpu) {
                return timestamp;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        // unsafe {
        //     let disjoint: D3D11_QUERY_DATA_TIMESTAMP_DISJOINT =
        //         gpu.wait_for_query_data(&self.disjoint);

        //     let start: u64 = gpu.wait_for_query_data(&self.start);
        //     let end: u64 = gpu.wait_for_query_data(&self.end);

        //     GpuTimestampRange {
        //         label: self.label.clone(),
        //         disjoint: disjoint.Disjoint.into(),
        //         frequency: disjoint.Frequency,
        //         start,
        //         end,
        //     }
        // }
    }
}

#[derive(Debug)]
pub struct GpuTimestampRange {
    pub label: String,

    /// If true, indicates that something happened during the disjoint query that caused the timestamp counter to become unstable (throttling, power saving etc.), making the timestamp query unreliable.
    pub disjoint: bool,
    pub frequency: u64,
    pub start: u64,
    pub end: u64,
}

impl GpuTimestampRange {
    pub fn to_miliseconds_f32(&self) -> f32 {
        let duration = self.end - self.start;
        let duration_s = duration as f64 / self.frequency as f64;
        duration_s as f32 * 1000.0
    }
}

impl GpuContext {
    pub fn begin_event_span<D: AsRef<str>>(&self, name: &str, data: D) -> GpuEventGuard {
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

    pub fn create_query(&self, query: D3D11_QUERY) -> ID3D11Query {
        try_out_ptr(|out| unsafe {
            self.device.CreateQuery(
                &D3D11_QUERY_DESC {
                    Query: query,
                    MiscFlags: 0,
                },
                Some(out),
            )
        })
        .expect("Failed to create query")
    }

    pub unsafe fn get_query_data<T: Sized + Default>(
        &self,
        query: &ID3D11Query,
    ) -> windows::core::Result<Option<T>> {
        let mut data = T::default();

        let result = (windows::core::Interface::vtable(self.context()).GetData)(
            windows::core::Interface::as_raw(self.context()),
            query.as_raw(),
            &mut data as *mut T as *mut _,
            std::mem::size_of::<T>() as u32,
            0,
        );

        result.ok()?;

        Ok(match result {
            S_OK => Some(data),
            S_FALSE => None,
            _ => None,
        })
    }

    // pub unsafe fn wait_for_query_data<T: Sized + Default>(&self, query: &ID3D11Query) -> T {
    //     loop {
    //         if let Some(data) = self
    //             .get_query_data(query)
    //             .expect("Failed to retrieve query data")
    //         {
    //             return data;
    //         }
    //         std::thread::sleep(Duration::from_millis(1));
    //     }
    // }

    pub fn begin_profile_span(&self, name: &str) -> GpuProfilingGuard {
        let disjoint = self.create_query(D3D11_QUERY_TIMESTAMP_DISJOINT);
        let start = self.create_query(D3D11_QUERY_TIMESTAMP);
        let end = self.create_query(D3D11_QUERY_TIMESTAMP);

        unsafe {
            self.context().Begin(&disjoint);
            self.context().End(&start);
        }

        self.pending_timestamp_queries
            .lock()
            .push(PendingGpuTimestampRange {
                label: name.to_string(),
                disjoint: disjoint.clone(),
                start,
                end: end.clone(),
            });

        GpuProfilingGuard {
            disjoint,
            end,
            context: self.context.clone(),
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
/// Like gpu_event, but begins a set of timestamp queries as well
macro_rules! gpu_profile_event {
    ($gpu:expr, $name:expr) => {
        let __gpu_profileguard = $gpu.begin_profile_span($name);
        gpu_event!($gpu, $name);
    };
}

#[macro_export]
macro_rules! gpu_event {
    ($gpu:expr, $name:expr) => {
        gpu_event!($gpu, $name, "")
    };
    ($gpu:expr, $name:expr, $data:expr) => {
        let __gpu_eventguard = $gpu.begin_event_span($name, $data);
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
