use crate::errors;
use crate::events::JobStatus;
use tokio::sync::oneshot;

pub enum WorkerMessage {
    GetStatus {
        response: oneshot::Sender<errors::Result<JobStatus>>,
    },
    Stop,
}
