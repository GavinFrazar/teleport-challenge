use uuid::Uuid;

mod actor;
mod messages;

/// A `JobCoordinator` which provides functionality for managing jobs and querying job state.
///
/// This struct is actually an actor handle, the real work is done in the actor spawned by `JobCoordinator::new`,
/// but from the user perspective all that matters is that this struct provides methods for managing jobs.
/// The actor-handle abstraction allows this struct to be cloned freely in a multi-thread async context,
/// without requiring an Arc<Mutex> or any other means of synchronization.
#[derive(Clone)]
pub struct JobCoordinator {}

pub type JobId = Uuid;
pub type JobStatus = (); // TODO

impl JobCoordinator {
    pub fn new() -> Self {
        todo!()
    }

    /// TODO: make these args more generic
    pub async fn start_job(
        &self,
        cmd: String,
        args: Vec<String>,
        dir: String,
        envs: Vec<(String, String)>,
    ) -> JobId {
        todo!()
    }

    pub async fn stop_job(&self, job_id: JobId) {
        todo!()
    }

    pub async fn get_job_status(&self, job_id: JobId) -> JobStatus {
        todo!()
    }
}
