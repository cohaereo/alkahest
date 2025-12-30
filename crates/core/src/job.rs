use lazy_static::lazy_static;
pub use potassium;
use potassium::SchedulerConfiguration;

lazy_static! {
    pub static ref SCHEDULER: potassium::Scheduler = potassium::Scheduler::new(&{
        let mut config = SchedulerConfiguration::default();
        config
            .workers
            .iter_mut()
            .for_each(|w| w.priority = potassium::ThreadPriority::Highest);
        config
    });
}
