mod actor;
mod messages;

use self::{
    actor::JobCoordinator,
    messages::CoordinatorMessage::{
        self, GetStatus, StartJob, StopJob, StreamAll, StreamStderr, StreamStdout,
    },
};
use crate::events::JobStatus;
use crate::types::{Args, Dir, Envs, JobId, Program};
use crate::{errors, types::OutputBlob};
use std::io;
use tokio::sync::{mpsc, oneshot};

/// A `JobCoordinator` which provides functionality for managing jobs and querying job state.
///
/// This struct is actually an actor handle, the real work is done in the actor spawned by `JobCoordinator::new`,
/// but from the user perspective all that matters is that this struct provides methods for managing jobs.
/// The actor-handle abstraction allows this struct to be cloned freely in a multi-thread async context,
/// without requiring an Arc<Mutex> or any other means of synchronization.
#[derive(Clone)]
pub struct JobCoordinatorHandle {
    sender: mpsc::Sender<CoordinatorMessage>,
}

impl JobCoordinatorHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(32);
        JobCoordinator::spawn(receiver);
        Self { sender }
    }

    /// TODO: make these args more generic
    pub async fn start_job(
        &self,
        cmd: Program,
        args: Args,
        dir: Dir,
        envs: Envs,
    ) -> io::Result<JobId> {
        let (tx, rx) = oneshot::channel();
        let msg = StartJob {
            cmd,
            args,
            dir,
            envs,
            response: tx,
        };
        self.sender.send(msg).await.expect("JobCoordinator exited");
        rx.await.expect("JobCoordinator exited")
    }

    pub async fn stop_job(&self, job_id: JobId) -> errors::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(StopJob {
                job_id,
                response: tx,
            })
            .await
            .expect("JobCoordinator exited");
        rx.await.expect("JobCoordinator exited")
    }

    pub async fn get_job_status(&self, job_id: JobId) -> errors::Result<JobStatus> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(GetStatus {
                job_id,
                response: tx,
            })
            .await
            .expect("JobCoordinator exited");
        rx.await.expect("JobCoordinator exited")
    }

    pub async fn stream_stdout(
        &self,
        job_id: JobId,
    ) -> errors::Result<mpsc::UnboundedReceiver<OutputBlob>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(StreamStdout {
                job_id,
                response: tx,
            })
            .await
            .expect("JobCoordinator exited");
        rx.await.expect("JobCoordinator exited")
    }

    pub async fn stream_stderr(
        &self,
        job_id: JobId,
    ) -> errors::Result<mpsc::UnboundedReceiver<OutputBlob>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(StreamStderr {
                job_id,
                response: tx,
            })
            .await
            .expect("JobCoordinator exited");
        rx.await.expect("JobCoordinator exited")
    }

    pub async fn stream_all(
        &self,
        job_id: JobId,
    ) -> errors::Result<mpsc::UnboundedReceiver<OutputBlob>> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(StreamAll {
                job_id,
                response: tx,
            })
            .await
            .expect("JobCoordinator exited");
        rx.await.expect("JobCoordinator exited")
    }
}
