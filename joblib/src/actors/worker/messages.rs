use crate::error;
use crate::events::JobStatus;
use tokio::sync::oneshot;

pub enum WorkerMessage {
    GetStatus {
        response: oneshot::Sender<error::Result<JobStatus>>,
    },
    Stop {
        response: oneshot::Sender<error::Result<()>>,
    },
}
