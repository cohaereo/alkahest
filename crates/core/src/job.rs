use lazy_static::lazy_static;
pub use potassium;

lazy_static! {
    pub static ref SCHEDULER: potassium::Scheduler = potassium::Scheduler::new();
}

// pub fn job_builder<'a>(
//     name: impl Into<potassium::util::SharedString>,
// ) -> potassium::JobBuilder<'a> {
//     SCHEDULER.job_builder(name)
// }
