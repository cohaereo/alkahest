use std::{sync::Arc, time::Duration};

use alkahest_core::job::{
    potassium::{JobHandle, WaitResult},
    SCHEDULER,
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
            .filter(|n| n.visible)
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

    // pub fn submit_stage_multi(&self, cmd: &mut CommandList, stage: RenderStage, job_count: usize) {
    //     if !ConVars::get_flag("render.threaded_submit") {
    //         self.submit_stage(cmd, stage, FeatureRendererSubscription::all());
    //         return;
    //     }

    //     let job_count = job_count.max(1);
    //     profiling::scope!(
    //         "submit_stage_multi",
    //         &format!("stage={stage:?} jobs={job_count}")
    //     );

    //     let initial_state = Arc::new(GpuState::backup(cmd));

    //     let mut jobs = Vec::new();

    //     let node_count = self.frame_packet.read().frame_nodes.len();
    //     for i in 0..job_count {
    //         let node_start = (i * node_count) / job_count;
    //         let node_end = ((i + 1) * node_count) / job_count;

    //         // let mut job_cmd = self.gpu.create_command_list();
    //         // initial_state.restore(&mut job_cmd);
    //         let j = self.submit_jobs.submit_job(SubmitJobDesc {
    //             node_range: node_start..node_end,
    //             stage,
    //             initial_state: Some(initial_state.clone()),
    //             index: i,
    //         });
    //         jobs.push(j);
    //     }

    //     let mut resolved = vec![];
    //     loop {
    //         for j in &jobs {
    //             if let Some(command_list) = self.submit_jobs.poll_job(*j) {
    //                 // We only need to restore state on the last job
    //                 profiling::scope!("execute_command_list", &format!("job={j:?}"));
    //                 cmd.execute_command_list(&command_list, false);

    //                 resolved.push(j);
    //             }
    //         }

    //         if resolved.len() == jobs.len() {
    //             break;
    //         }

    //         // std::thread::yield_now();
    //     }

    //     // for j in jobs {
    //     //     if let Some(command_list) = self.submit_jobs.await_job(j) {
    //     //         // We only need to restore state on the last job
    //     //         profiling::scope!("execute_command_list", &format!("job={:?}", j));
    //     //         cmd.execute_command_list(&command_list.finish_command_list(false).unwrap(), false);
    //     //     } else {
    //     //         error!("Submit job {j:?} got dropped?");
    //     //     }
    //     // }

    //     initial_state.restore(cmd);
    // }
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

// pub struct SubmitJobManager {
//     next_job_id: AtomicUsize,
//     pending_jobs: RwLock<HashMap<SubmitJobId, Option<JobResult>>>,
//     result_rx: Receiver<(SubmitJobId, JobResult)>,
//     job_tx: Sender<(SubmitJobId, SubmitJobDesc)>,
//     thread_handles: Vec<JoinHandle<()>>,
// }

// impl SubmitJobManager {
//     pub fn new(gpu: &Arc<Gpu>, thread_count: usize) -> Self {
//         let (job_tx, job_rx) = unbounded();
//         let (result_tx, result_rx) = unbounded();
//         let mut thread_handles = Vec::new();
//         let nproc = gdt_cpus::num_logical_cores().unwrap_or(1);

//         for i in 0..thread_count {
//             let job_rx = job_rx.clone();
//             let result_tx = result_tx.clone();
//             let gpu = gpu.clone();

//             let handle = std::thread::Builder::new()
//                 .name(format!("render_submit_{i}"))
//                 .spawn(move || {
//                     if let Err(e) = gdt_cpus::set_thread_priority(gdt_cpus::ThreadPriority::Highest)
//                     {
//                         error!("Failed to set submit thread priority: {e}");
//                     }
//                     if let Err(e) = gdt_cpus::pin_thread_to_core(nproc - i % nproc - 1) {
//                         error!("Failed to pin submit thread to core: {e}");
//                     }

//                     let mut command_list = None;
//                     while let Ok((job_id, job_desc)) = job_rx.recv() {
//                         // Workaround so we don't create a new command list before the first job
//                         // This is because creating a command list creates a reference to the *renderer's* extern container,
//                         // which we can't do until the renderer is fully initialized (which happens after the job manager is created)
//                         if command_list.is_none() {
//                             command_list = Some(gpu.create_command_list());
//                         }
//                         let command_list = command_list.as_mut().unwrap();
//                         let SubmitJobDesc {
//                             node_range,
//                             stage,
//                             initial_state,
//                             index,
//                         } = job_desc;
//                         let _ = index;
//                         profiling::scope!(
//                             "threaded_submit_job",
//                             &format!("job_id={job_id:?} index={index} stage={stage:?}")
//                         );

//                         if let Some(initial_state) = initial_state {
//                             initial_state.restore(command_list);
//                         }

//                         let renderer = Renderer::instance();
//                         renderer.submit_stage_range(
//                             command_list,
//                             node_range,
//                             stage,
//                             FeatureRendererSubscription::all(),
//                         );

//                         result_tx
//                             .send((
//                                 job_id,
//                                 JobResult {
//                                     cmd: command_list.finish_command_list(false).unwrap(),
//                                 },
//                             ))
//                             .unwrap();
//                     }
//                 })
//                 .expect("Failed to spawn submit thread");

//             thread_handles.push(handle);
//         }

//         Self {
//             next_job_id: AtomicUsize::new(0),
//             job_tx,
//             result_rx,
//             pending_jobs: RwLock::new(HashMap::new()),
//             thread_handles,
//         }
//     }

//     pub fn submit_job(&self, job_desc: SubmitJobDesc) -> SubmitJobId {
//         let job_id = SubmitJobId(self.next_job_id.fetch_add(1, Ordering::SeqCst));
//         self.pending_jobs.write().insert(job_id, None);
//         self.job_tx.send((job_id, job_desc)).unwrap();
//         job_id
//     }

//     fn collect_jobs(&self) {
//         if self.result_rx.is_empty() {
//             return;
//         }

//         let mut pending_jobs = self.pending_jobs.write();
//         for (job_id, result) in self.result_rx.try_iter() {
//             pending_jobs.insert(job_id, Some(result));
//         }
//     }

//     // pub fn await_job(&self, job_id: SubmitJobId) -> Option<CommandList> {
//     //     let mut jobs = self.jobs.write();
//     //     if !jobs.contains_key(&job_id.0) {
//     //         // Job id doesn't exist or has already been awaited
//     //         return None;
//     //     }

//     //     loop {
//     //         if self.thread_handles.iter().any(|c| c.is_finished()) {
//     //             panic!("Submission thread died");
//     //         }

//     //         // Check if the job has been completed, if not, skip to the next iteration
//     //         if jobs.get(&job_id.0).unwrap().is_some() {
//     //             let res = jobs.remove(&job_id.0).unwrap().unwrap();
//     //             return Some(res.cmd);
//     //         }
//     //         // self.condvar.wait_for(&mut jobs, Duration::from_millis(100));
//     //     }
//     // }

//     pub fn poll_job(&self, job_id: SubmitJobId) -> Option<d3d11::CommandList> {
//         self.collect_jobs();

//         let jobs_read = self.pending_jobs.read();
//         if !jobs_read.contains_key(&job_id) {
//             // Job id doesn't exist or has already been awaited
//             return None;
//         }

//         if jobs_read.get(&job_id).unwrap().is_some() {
//             drop(jobs_read);
//             let res = self.pending_jobs.write().remove(&job_id).unwrap().unwrap();
//             return Some(res.cmd);
//         }
//         None
//     }
// }

// impl Drop for SubmitJobManager {
//     fn drop(&mut self) {
//         // Replace the sender with a dummy sender to signal the threads to exit by dropping the channel
//         self.job_tx = unbounded().0;
//         for handle in self.thread_handles.drain(..) {
//             handle.join().expect("Failed to join thread");
//         }
//     }
// }
