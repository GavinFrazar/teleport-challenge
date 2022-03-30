use crate::errors;
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
        response: oneshot::Sender<errors::Result<()>>,
    },
    GetStatus {
        job_id: JobId,
        response: oneshot::Sender<errors::Result<JobStatus>>,
    },
    GetOutput {
        request: StreamRequest,
        response: oneshot::Sender<mpsc::UnboundedReceiver<OutputBlob>>,
    },
}

#[derive(Debug)]
pub enum StreamRequest {
    StreamStdout {
        response: mpsc::UnboundedSender<OutputBlob>,
        job_id: JobId,
    },
    StreamStderr {
        response: mpsc::UnboundedSender<OutputBlob>,
        job_id: JobId,
    },
    StreamAll {
        response: mpsc::UnboundedSender<OutputBlob>,
        job_id: JobId,
    },
}
