use std::{collections::VecDeque, sync::Arc, time::Instant};

use anyhow::Context;
use d3d11::{
    DeviceContext, Query,
    query::{D3D11_QUERY_DATA_TIMESTAMP_DISJOINT, QueryDesc},
};
use parking_lot::Mutex;

use crate::Gpu;

const MAX_PENDING_FRAMES: usize = 4;

#[derive(Clone, Debug)]
pub struct ProfileScope {
    pub name: String,
    pub depth: u32,
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
    depth: u32,
}

struct FrameQueries {
    queries: Vec<TimestampQuery>,
    frame_index: u64,
}

struct ProfilerState {
    gpu: Arc<Gpu>,
    enabled: bool,

    current_frame: FrameQueries,
    pending_frames: VecDeque<FrameQueries>,
    /// Last 10 completed frame results.
    results: Vec<Vec<ProfileScope>>,

    frame_index: u64,

    scope_depth: u32,
}

#[derive(Clone)]
pub struct D3D11Profiler {
    state: Arc<Mutex<ProfilerState>>,
}

impl D3D11Profiler {
    pub fn new(gpu: &Arc<Gpu>) -> Self {
        let state = ProfilerState {
            gpu: gpu.clone(),
            enabled: true,
            current_frame: FrameQueries {
                queries: Vec::new(),
                frame_index: 0,
            },
            pending_frames: VecDeque::with_capacity(MAX_PENDING_FRAMES),
            results: Vec::new(),
            frame_index: 0,
            scope_depth: 0,
        };

        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub fn scope(&self, context: &DeviceContext, name: impl Into<String>) -> ProfileScopeGuard {
        if !self.state.lock().enabled {
            ProfileScopeGuard::none(self.clone())
        } else {
            ProfileScopeGuard::new(self.clone(), context, name.into())
        }
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

        let depth = state.scope_depth;
        state.current_frame.queries.push(TimestampQuery {
            context: context.clone(),
            name,
            start_query,
            end_query,
            disjoint_query,
            cpu_start,
            cpu_duration_us: 0.0,
            depth,
        });
        state.scope_depth += 1;

        Ok(ScopeHandle { index })
    }

    fn end_scope(&self, handle: ScopeHandle) -> anyhow::Result<()> {
        let ProfilerState {
            enabled,
            current_frame,
            scope_depth,
            ..
        } = &mut *self.state.lock();
        if !*enabled {
            return Ok(());
        }

        if handle.index >= current_frame.queries.len() {
            anyhow::bail!("Invalid scope handle");
        }

        let query = &mut current_frame.queries[handle.index];

        query.context.end(&query.end_query);

        query.context.end(&query.disjoint_query);

        query.cpu_duration_us = query.cpu_start.elapsed().as_secs_f64() * 1_000_000.0;

        *scope_depth = scope_depth
            .checked_sub(1)
            .context("Mismatched GPU profiling scope begin/end")?;

        Ok(())
    }

    pub fn end_frame(&self) {
        let ProfilerState {
            gpu,
            enabled,
            current_frame,
            pending_frames,
            results: all_results,
            frame_index,
            scope_depth,
            ..
        } = &mut *self.state.lock();
        if !*enabled {
            return;
        }

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
                all_results.push(results);
                while all_results.len() > 60 {
                    all_results.remove(0);
                }
                pending_frames.pop_front();
            }
        } else {
            warn!("No pending profiling frames");
        }

        if *scope_depth != 0 {
            warn!("Some GPU profiling scopes were not properly ended before frame end");
            *scope_depth = 0;
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
                depth: query.depth,
                cpu_duration_us: query.cpu_duration_us,
                gpu_duration_us,
            });
        }

        Ok(results)
    }

    pub fn get_results_string(&self) -> String {
        if !self.state.lock().enabled {
            return "GPU profiling is disabled".to_string();
        }

        let results_raw = self.state.lock().results.clone();
        if results_raw.is_empty() {
            return "No profiling data available yet".to_string();
        }

        let mut results = Vec::new();
        for scope in &results_raw[results_raw.len() - 1] {
            // Average over last 10 frames
            let mut total_cpu = 0.0;
            let mut total_gpu = 0.0;
            let mut count = 0;
            for frame in results_raw.iter().rev() {
                if let Some(s) = frame
                    .iter()
                    .find(|s| s.name == scope.name && s.depth == scope.depth)
                {
                    total_cpu += s.cpu_duration_us;
                    total_gpu += s.gpu_duration_us;
                    count += 1;
                }
            }

            results.push(ProfileScope {
                name: scope.name.clone(),
                depth: scope.depth,
                cpu_duration_us: total_cpu / count as f64,
                gpu_duration_us: total_gpu / count as f64,
            });
        }

        let mut output = String::new();
        output.push_str(&format!(
            "{:<30} {:>12} {:>12}\n",
            "Scope", "CPU (ms)", "GPU (ms)"
        ));
        output.push_str(&"-".repeat(56));
        output.push('\n');

        let mut longest_duration_cpu = 0f64;
        let mut longest_duration_gpu = 0f64;
        for scope in results {
            let scope_name = if scope.depth > 0 {
                format!("{}{}", "  ".repeat(scope.depth as usize), scope.name)
            } else {
                scope.name.clone()
            };
            output.push_str(&format!(
                "{:<30} {:>12.3} {:>12.3}\n",
                scope_name,
                scope.cpu_duration_us / 1000.0,
                scope.gpu_duration_us / 1000.0
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
    fn none(profiler: D3D11Profiler) -> Self {
        Self {
            profiler,
            handle: None,
        }
    }

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
