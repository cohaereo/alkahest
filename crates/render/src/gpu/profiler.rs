use std::{collections::VecDeque, sync::Arc, time::Instant};

use anyhow::Context;
use d3d11::{
    query::{QueryDesc, D3D11_QUERY_DATA_TIMESTAMP_DISJOINT},
    DeviceContext, Query,
};
use parking_lot::Mutex;

use crate::Gpu;

const MAX_PENDING_FRAMES: usize = 4;

#[derive(Clone, Debug)]
pub struct ProfileScope {
    pub name: String,
    pub cpu_duration_us: f64,
    pub gpu_duration_us: f64,
}

struct TimestampQuery {
    context: DeviceContext,
    name: String,
    start_query: Query,
    end_query: Query,
    disjoint_query: Query,
    cpu_start: Instant,
    cpu_duration_us: f64,
}

struct FrameQueries {
    queries: Vec<TimestampQuery>,
    frame_index: u64,
}

struct ProfilerState {
    gpu: Arc<Gpu>,

    current_frame: FrameQueries,

    pending_frames: VecDeque<FrameQueries>,

    last_results: Vec<ProfileScope>,

    frame_index: u64,
}

#[derive(Clone)]
pub struct D3D11Profiler {
    state: Arc<Mutex<ProfilerState>>,
}

impl D3D11Profiler {
    pub fn new(gpu: &Arc<Gpu>) -> Self {
        let state = ProfilerState {
            gpu: gpu.clone(),
            current_frame: FrameQueries {
                queries: Vec::new(),
                frame_index: 0,
            },
            pending_frames: VecDeque::with_capacity(MAX_PENDING_FRAMES),
            last_results: Vec::new(),
            frame_index: 0,
        };

        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub fn scope(&self, context: &DeviceContext, name: impl Into<String>) -> ProfileScopeGuard {
        ProfileScopeGuard::new(self.clone(), context, name.into())
    }

    fn start_scope(&self, name: String, context: &DeviceContext) -> anyhow::Result<ScopeHandle> {
        let mut state = self.state.lock();

        let device = &state.gpu.device;

        let start_query = device
            .create_query(&QueryDesc::timestamp())
            .context("Failed to create start query")?;

        let end_query = device
            .create_query(&QueryDesc::timestamp())
            .context("Failed to create end query")?;

        let disjoint_query = device
            .create_query(&QueryDesc::timestamp_disjoint())
            .context("Failed to create disjoint query")?;

        context.begin(&disjoint_query);

        context.end(&start_query);

        let cpu_start = Instant::now();
        let index = state.current_frame.queries.len();

        state.current_frame.queries.push(TimestampQuery {
            context: context.clone(),
            name,
            start_query,
            end_query,
            disjoint_query,
            cpu_start,
            cpu_duration_us: 0.0,
        });

        Ok(ScopeHandle { index })
    }

    fn end_scope(&self, handle: ScopeHandle) -> anyhow::Result<()> {
        let ProfilerState { current_frame, .. } = &mut *self.state.lock();

        if handle.index >= current_frame.queries.len() {
            anyhow::bail!("Invalid scope handle");
        }

        let query = &mut current_frame.queries[handle.index];

        query.context.end(&query.end_query);

        query.context.end(&query.disjoint_query);

        query.cpu_duration_us = query.cpu_start.elapsed().as_secs_f64() * 1_000_000.0;

        Ok(())
    }

    pub fn end_frame(&self) {
        let ProfilerState {
            gpu,
            current_frame,
            pending_frames,
            last_results,
            frame_index,
            ..
        } = &mut *self.state.lock();

        let mut frame = FrameQueries {
            queries: Vec::new(),
            frame_index: *frame_index,
        };
        std::mem::swap(&mut frame, current_frame);

        pending_frames.push_back(frame);
        *frame_index += 1;

        while pending_frames.len() > MAX_PENDING_FRAMES {
            pending_frames.pop_front();
        }

        if let Some(oldest_frame) = pending_frames.front() {
            if let Ok(results) = Self::try_get_frame_results(&gpu.context(), oldest_frame) {
                *last_results = results;
                pending_frames.pop_front();
            }
        } else {
            warn!("No pending profiling frames");
        }
    }

    fn try_get_frame_results(
        context: &DeviceContext,
        frame: &FrameQueries,
    ) -> anyhow::Result<Vec<ProfileScope>> {
        let mut results = Vec::new();

        for query in &frame.queries {
            let start_data = match unsafe { context.get_data::<u64>(&query.start_query, false) } {
                d3d11::GetDataResult::Ok(o) => o,
                d3d11::GetDataResult::Pending => anyhow::bail!("Start query not ready"),
                d3d11::GetDataResult::Error(error) => {
                    anyhow::bail!(format!("Failed to get start query data: {error:?}"))
                }
            };

            let end_data = match unsafe { context.get_data::<u64>(&query.end_query, false) } {
                d3d11::GetDataResult::Ok(o) => o,
                d3d11::GetDataResult::Pending => anyhow::bail!("Start query not ready"),
                d3d11::GetDataResult::Error(error) => {
                    anyhow::bail!(format!("Failed to get start query data: {error:?}"))
                }
            };

            let disjoint_data = match unsafe {
                context
                    .get_data::<D3D11_QUERY_DATA_TIMESTAMP_DISJOINT>(&query.disjoint_query, false)
            } {
                d3d11::GetDataResult::Ok(o) => o,
                d3d11::GetDataResult::Pending => {
                    anyhow::bail!("Disjoint query not ready")
                }
                d3d11::GetDataResult::Error(error) => {
                    anyhow::bail!(format!("Failed to get disjoint query data: {error:?}"))
                }
            };

            if disjoint_data.Disjoint.as_bool() {
                continue;
            }

            let ticks = end_data.saturating_sub(start_data);
            let gpu_duration_us = if disjoint_data.Frequency > 0 {
                (ticks as f64 / disjoint_data.Frequency as f64) * 1_000_000.0
            } else {
                0.0
            };

            results.push(ProfileScope {
                name: query.name.clone(),
                cpu_duration_us: query.cpu_duration_us,
                gpu_duration_us,
            });
        }

        Ok(results)
    }

    pub fn get_results(&self) -> Vec<ProfileScope> {
        let state = self.state.lock();
        state.last_results.clone()
    }

    pub fn get_results_string(&self) -> String {
        let results = self.get_results();

        if results.is_empty() {
            return "No profiling data available yet".to_string();
        }

        let mut output = String::new();
        output.push_str(&format!(
            "{:<30} {:>12} {:>12}\n",
            "Scope", "CPU (µs)", "GPU (µs)"
        ));
        output.push_str(&"-".repeat(56));
        output.push('\n');

        let mut longest_duration_cpu = 0f64;
        let mut longest_duration_gpu = 0f64;
        for scope in results {
            output.push_str(&format!(
                "{:<30} {:>12.1} {:>12.1}\n",
                scope.name, scope.cpu_duration_us, scope.gpu_duration_us
            ));
            longest_duration_cpu = longest_duration_cpu.max(scope.cpu_duration_us);
            longest_duration_gpu = longest_duration_gpu.max(scope.gpu_duration_us);
        }

        output.push_str(&"-".repeat(56));
        output.push('\n');
        output.push_str(&format!(
            "{:<30} {:>12.1} {:>12.1}\n",
            "Potential FPS",
            if longest_duration_cpu > 0.0 {
                1_000_000.0 / longest_duration_cpu
            } else {
                f64::INFINITY
            },
            if longest_duration_gpu > 0.0 {
                1_000_000.0 / longest_duration_gpu
            } else {
                f64::INFINITY
            }
        ));

        output
    }
}

struct ScopeHandle {
    index: usize,
}

pub struct ProfileScopeGuard {
    profiler: D3D11Profiler,
    handle: Option<ScopeHandle>,
}

impl ProfileScopeGuard {
    fn new(profiler: D3D11Profiler, context: &DeviceContext, name: String) -> Self {
        let handle = profiler.start_scope(name, context).ok();
        Self { profiler, handle }
    }

    /// Executes the provided closure within the scope of this profiling guard.
    pub fn span<F: FnOnce()>(self, f: F) {
        f();
    }
}

impl Drop for ProfileScopeGuard {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = self.profiler.end_scope(handle);
        }
    }
}
