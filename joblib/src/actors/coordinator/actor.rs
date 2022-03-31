use super::messages::CoordinatorMessage;
use crate::actors::{broadcaster::BroadcasterHandle, worker::WorkerHandle};
use crate::errors::{self, JobError};
use crate::events::JobStatus;
use crate::types::{Args, Dir, Envs, JobId, OutputBlob, Program};
use std::{collections::HashMap, io};
use tokio::sync::{mpsc, oneshot};

pub struct JobCoordinator {
    inbox: mpsc::Receiver<CoordinatorMessage>,
    workers: HashMap<JobId, WorkerHandle>,
    broadcasters: HashMap<JobId, BroadcasterHandle>,
}

impl JobCoordinator {
    pub fn spawn(inbox: mpsc::Receiver<CoordinatorMessage>) {
        let actor = Self {
            inbox,
            workers: HashMap::new(),
            broadcasters: HashMap::new(),
        };
        tokio::spawn(async move { actor.run().await });
    }

    async fn run(mut self) {
        use self::CoordinatorMessage::*;
        while let Some(msg) = self.inbox.recv().await {
            match msg {
                StartJob {
                    cmd,
                    args,
                    dir,
                    envs,
                    response,
                } => {
                    self.start_job(cmd, args, dir, envs, response);
                }
                StopJob { job_id, response } => {
                    self.stop_job(job_id, response);
                }
                GetStatus { job_id, response } => {
                    self.get_job_status(job_id, response);
                }
                StreamStdout { job_id, response } => {
                    self.stream_stdout(job_id, response);
                }
                StreamStderr { job_id, response } => {
                    self.stream_stderr(job_id, response);
                }
                StreamAll { job_id, response } => {
                    self.stream_all(job_id, response);
                }
            }
        }
    }

    fn start_job(
        &mut self,
        cmd: Program,
        args: Args,
        dir: Dir,
        envs: Envs,
        response: oneshot::Sender<io::Result<JobId>>,
    ) {
        let (output_tx, output_rx) = mpsc::unbounded_channel(); // channel for piping child process output

        // spawn the worker with the sending end of the output channel.
        match WorkerHandle::spawn(output_tx, cmd, args, dir, envs) {
            Ok(worker) => {
                // broadcaster will receive events from the child process via this receiver channel
                let broadcaster = BroadcasterHandle::spawn(output_rx);

                let job_id = uuid::Uuid::new_v4();
                self.workers.insert(job_id, worker);
                self.broadcasters.insert(job_id, broadcaster);
                let _ = response.send(Ok(job_id));
            }
            Err(e) => {
                // if spawning child process fails, we don't insert the worker broadcaster handles in our map.
                // no actors spawn in this case.
                let _ = response.send(Err(e));
            }
        }
    }

    fn stop_job(&mut self, job_id: JobId, response: oneshot::Sender<errors::Result<()>>) {
        if let Some(worker) = self.workers.get(&job_id) {
            worker.stop();
            let _ = response.send(Ok(()));
        } else {
            let _ = response.send(Err(JobError::NotFound));
        }
    }

    fn get_job_status(
        &mut self,
        job_id: JobId,
        response: oneshot::Sender<errors::Result<JobStatus>>,
    ) {
        if let Some(worker) = self.workers.get(&job_id) {
            worker.get_status(response);
        } else {
            let _ = response.send(Err(JobError::NotFound));
        }
    }

    fn stream_stdout(
        &mut self,
        job_id: JobId,
        response: oneshot::Sender<errors::Result<mpsc::UnboundedReceiver<OutputBlob>>>,
    ) {
        let (subscriber_tx, subscriber_rx) = mpsc::unbounded_channel();
        if let Some(broadcaster) = self.broadcasters.get(&job_id) {
            broadcaster.stream_stdout(subscriber_tx);
            let _ = response.send(Ok(subscriber_rx));
        } else {
            let _ = response.send(Err(JobError::NotFound));
        }
    }

    fn stream_stderr(
        &mut self,
        job_id: JobId,
        response: oneshot::Sender<errors::Result<mpsc::UnboundedReceiver<OutputBlob>>>,
    ) {
        let (subscriber_tx, subscriber_rx) = mpsc::unbounded_channel();
        if let Some(broadcaster) = self.broadcasters.get(&job_id) {
            broadcaster.stream_stderr(subscriber_tx);
            let _ = response.send(Ok(subscriber_rx));
        } else {
            let _ = response.send(Err(JobError::NotFound));
        }
    }

    fn stream_all(
        &mut self,
        job_id: JobId,
        response: oneshot::Sender<errors::Result<mpsc::UnboundedReceiver<OutputBlob>>>,
    ) {
        let (subscriber_tx, subscriber_rx) = mpsc::unbounded_channel();
        if let Some(broadcaster) = self.broadcasters.get(&job_id) {
            broadcaster.stream_all(subscriber_tx);
            let _ = response.send(Ok(subscriber_rx));
        } else {
            let _ = response.send(Err(JobError::NotFound));
        }
    }
}
