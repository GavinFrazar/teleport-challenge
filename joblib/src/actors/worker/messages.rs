use crate::events::JobStatus;
use tokio::sync::oneshot;

pub enum WorkerMessage {
    GetStatus {
        response: oneshot::Sender<JobStatus>,
    },
    Stop,
}
