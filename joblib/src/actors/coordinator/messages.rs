use crate::error;
use crate::events::JobStatus;
use crate::types::{Args, Dir, Envs, JobId, OutputBlob, Program};
use std::io;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum CoordinatorMessage {
    StartJob {
        cmd: Program,
        args: Args,
        dir: Dir,
        envs: Envs,
        response: oneshot::Sender<io::Result<JobId>>,
    },
    StopJob {
        job_id: JobId,
        response: oneshot::Sender<error::Result<()>>,
    },
    GetStatus {
        job_id: JobId,
        response: oneshot::Sender<error::Result<JobStatus>>,
    },
    StreamStdout {
        job_id: JobId,
        response: oneshot::Sender<error::Result<mpsc::UnboundedReceiver<OutputBlob>>>,
    },
    StreamStderr {
        job_id: JobId,
        response: oneshot::Sender<error::Result<mpsc::UnboundedReceiver<OutputBlob>>>,
    },
    StreamAll {
        job_id: JobId,
        response: oneshot::Sender<error::Result<mpsc::UnboundedReceiver<OutputBlob>>>,
    },
}
