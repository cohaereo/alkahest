use std::{sync::Arc, time::Duration};

use alkahest_core::job::{
    SCHEDULER,
    potassium::{JobHandle, WaitResult},
};
use alkahest_data::tfx::{FeatureRendererSubscription, RenderStage};

use super::Renderer;
use crate::{
    gpu::{command_list::CommandList, state::GpuState},
    util::threading::CommandListSetId,
};

impl Renderer {
    pub fn submit_stage_range(
        &self,
        cmd: &mut CommandList,
        frame_node_range: std::ops::Range<usize>,
        stage: RenderStage,
        mut features: FeatureRendererSubscription,
    ) {
        features = self.active_feature_renderers.load().intersection(features);
        if features.is_empty() {
            return;
        }
        profiling::scope!("submit_stage", &format!("stage={stage:?}"));

        for obj in self.frame_packet.read().frame_nodes[frame_node_range]
            .iter()
            // TODO(cohae): ShadowGenerate workaround until lights have their own culling
            .filter(|n| n.visible || stage == RenderStage::ShadowGenerate)
        {
            if let Some(render_object) = self
                .objects
                .read()
                .get(obj.render_object_handle.into())
                .filter(|p| p.stages.is_subscribed(stage) && features.is_subscribed(p.feature_type))
            {
                render_object.submit(cmd, stage);
            }
        }
    }

    pub fn submit_stage(
        &self,
        cmd: &mut CommandList,
        stage: RenderStage,
        features: FeatureRendererSubscription,
    ) {
        profiling::scope!(
            "submit_stage",
            &format!("stage={stage:?}, features={features:?}")
        );
        self.submit_stage_range(
            cmd,
            0..self.frame_packet.read().frame_nodes.len(),
            stage,
            features,
        );
    }

    #[must_use]
    pub fn submit_stage_parallel(
        self: &Arc<Self>,
        cmd: &mut CommandList,
        stage: RenderStage,
        mut features: FeatureRendererSubscription,
    ) -> (JobHandle, CommandListSetId) {
        features = self.active_feature_renderers.load().intersection(features);
        // if features.is_empty() {
        //     return None;
        // }
        profiling::scope!("submit_stage_parallel", &format!("stage={stage:?}"));

        let cmd_set = unsafe { self.cmd_pool.begin(cmd) };
        let mut job_handles = Vec::new();
        for obj in self
            .frame_packet
            .read()
            .frame_nodes
            .iter()
            .filter(|n| n.visible)
        {
            if let Some(render_object) = self
                .objects
                .read()
                .get(obj.render_object_handle.into())
                .filter(|p| p.stages.is_subscribed(stage) && features.is_subscribed(p.feature_type))
            {
                render_object.submit_parallel(self, cmd_set, stage, &mut job_handles);
            }
        }

        let sync_job = SCHEDULER
            .job_builder("submit_stage_parallel_sync")
            .dependencies(job_handles)
            .spawn(|| {});

        (sync_job, cmd_set)
    }

    /// Submits the given render stage in parallel using multiple jobs, applying the results to the given command list.
    ///
    /// This function will block until all jobs have completed and the command list has been updated. `submit_stage_parallel` is often preferred to prevent wasting job threads while waiting.
    ///
    /// Note: The order in which objects are submitted is basically random, so this function should not be used when order matters. Use `submit_stage_parallel_linear` instead in those cases.
    pub fn submit_stage_parallel_apply(
        self: &Arc<Self>,
        cmd: &mut CommandList,
        stage: RenderStage,
        features: FeatureRendererSubscription,
    ) {
        let (sync_job, cmd_set) = self.submit_stage_parallel(cmd, stage, features);
        if sync_job.wait_timeout(Duration::from_millis(500)) == WaitResult::Timeout {
            // cohae: This is a leftover debug check from an issue in Potassium where jobs would deadlock sometimes due to faulty dependency tracking.
            // This should never be able to happen after potassium 0.3, but just in case, we keep this log here.
            error!("Deadlock detected: submit_stage_parallel_sync timed out");
        }

        self.cmd_pool.finish(cmd, cmd_set);
    }

    /// Submits the given render stage in parallel using multiple jobs
    ///
    /// Unlike `submit_stage_parallel`, this function guarantees that objects are submitted in linear order.
    ///
    /// This does come with a performance cost, but is still faster than single-threaded submission.
    ///
    /// This function does not use `submit_parallel` to generate jobs, but instead divides all the work into N jobs, where N is the number of worker threads.
    pub fn submit_stage_parallel_linear(
        self: &Arc<Self>,
        cmd: &mut CommandList,
        stage: RenderStage,
        mut features: FeatureRendererSubscription,
    ) {
        features = self.active_feature_renderers.load().intersection(features);
        profiling::scope!("submit_stage_parallel_linear", &format!("stage={stage:?}"));

        let cmd_sets: [CommandListSetId; 3] =
            std::array::from_fn(|_| unsafe { self.cmd_pool.begin(cmd) });
        let mut object_handles = Vec::new();
        for obj in self
            .frame_packet
            .read()
            .frame_nodes
            .iter()
            .filter(|n| n.visible)
        {
            if self
                .objects
                .read()
                .get(obj.render_object_handle.into())
                .filter(|p| p.stages.is_subscribed(stage) && features.is_subscribed(p.feature_type))
                .is_some()
            {
                object_handles.push(obj.render_object_handle);
            }
        }

        let object_handles = Arc::new(object_handles);

        let mut job_handles = Vec::new();

        // Divide work into N jobs, where N is the number of worker threads
        // TODO(cohae): the cmdlist pool only provides N command lists, but some threads have a lot more work than others, so some threads may take much longer than others, leading to idle time on the other threads.
        // Ideally we would allocate 2x/3x command lists for linear submission
        // *Alternatively*, we could just steal more than 1 command set
        let num_threads = SCHEDULER.num_workers();
        let num_jobs = num_threads * cmd_sets.len();
        let chunk_size = object_handles.len().div_ceil(num_jobs);
        for chunk_idx in 0..num_jobs {
            let renderer_clone = Arc::clone(self);
            let object_handles_clone = Arc::clone(&object_handles);
            let start_idx = chunk_idx * chunk_size;
            let end_idx = ((chunk_idx + 1) * chunk_size).min(object_handles_clone.len());
            if start_idx >= end_idx {
                continue;
            }

            let job_handle = SCHEDULER
                .job_builder("submit_stage_parallel_linear_chunk")
                .spawn(move || {
                    let cmd_set = cmd_sets[chunk_idx.div_floor(num_threads)];
                    let cmd = renderer_clone
                        .cmd_pool
                        .get_command_list_manual(cmd_set, chunk_idx % num_threads)
                        .expect("Invalid command list index");

                    for handle in &object_handles_clone[start_idx..end_idx] {
                        if let Some(render_object) = renderer_clone
                            .objects
                            .read()
                            .get((*handle).into())
                            .filter(|p| {
                                p.stages.is_subscribed(stage)
                                    && features.is_subscribed(p.feature_type)
                            })
                        {
                            render_object.submit(cmd, stage);
                        }
                    }
                });
            job_handles.push(job_handle);
        }

        let sync_job = SCHEDULER
            .job_builder("submit_stage_parallel_linear_sync")
            .dependencies(job_handles)
            .spawn(|| {});
        sync_job.wait();

        // CommandListPool::finish executes command lists in linear order
        for cmd_set in cmd_sets {
            self.cmd_pool.finish(cmd, cmd_set);
        }
    }
}

pub struct SubmitJobDesc {
    pub node_range: std::ops::Range<usize>,
    pub stage: RenderStage,
    pub initial_state: Option<Arc<GpuState>>,
    pub index: usize,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SubmitJobId(usize);

pub struct JobResult {
    pub cmd: d3d11::CommandList,
}
