use super::messages::{CoordinatorMessage, StreamRequest};
use crate::actors::{broadcaster::BroadcasterHandle, worker::WorkerHandle};
use crate::events::JobStatus;
use crate::types::{Args, Dir, Envs, JobId, OutputBlob, Program};
use std::collections::HashMap;
use std::io;
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
                GetOutput { request, response } => {
                    self.get_job_output(request, response);
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
        let job_id = uuid::Uuid::new_v4();
        let _ = response.send(Ok(job_id)); // ignore send error - caller is no longer interested
        todo!()
    }

    fn stop_job(&mut self, job_id: JobId, response: oneshot::Sender<io::Result<()>>) {
        todo!()
    }

    fn get_job_status(&mut self, job_id: JobId, response: oneshot::Sender<io::Result<JobStatus>>) {
        todo!()
    }

    fn get_job_output(
        &mut self,
        request: StreamRequest,
        response: oneshot::Sender<mpsc::UnboundedReceiver<OutputBlob>>,
    ) {
        todo!()
    }
}
