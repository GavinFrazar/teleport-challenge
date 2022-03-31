use super::messages::WorkerMessage;
use crate::error::Error as JobError;
use crate::events::JobStatus;
use crate::types::Output;

use bytes::BytesMut;
use futures::future::FutureExt;
use std::os::unix::process::ExitStatusExt;
use tokio::{
    io::AsyncReadExt,
    process::Child,
    select,
    sync::{mpsc, oneshot},
};

pub struct Actor {
    inbox: mpsc::UnboundedReceiver<WorkerMessage>,
    kill_tx: Option<oneshot::Sender<()>>,
    job_status: JobStatus,
}

impl Actor {
    pub fn spawn(
        inbox: mpsc::UnboundedReceiver<WorkerMessage>,
        broadcast_tx: mpsc::UnboundedSender<Output>,
        child: Child,
    ) {
        let (kill_tx, kill_rx) = oneshot::channel();
        tokio::spawn(async move {
            let actor = Self {
                inbox,
                kill_tx: Some(kill_tx),
                job_status: JobStatus::Running,
            };
            actor.run(broadcast_tx, kill_rx, child).await;
        });
    }

    pub async fn run(
        mut self,
        broadcast_tx: mpsc::UnboundedSender<Output>,
        kill_rx: oneshot::Receiver<()>,
        mut child: tokio::process::Child,
    ) {
        let (child_exit_tx, child_exit_rx) = oneshot::channel();
        // grab stdout and stderr, if they've been piped
        let maybe_stdout = child.stdout.take();
        let maybe_stderr = child.stderr.take();
        let mut kill_rx = kill_rx.fuse();

        // spawn the job
        tokio::spawn(async move {
            loop {
                select! {
                    // listen for a kill signal
                    _ = &mut kill_rx => {
                        let _ = child.kill().await;
                    }
                    // wait for child pid to finish and cleanup its resources
                    exit_status = child.wait() => {
                        let exit_status = exit_status.expect("child wait: io error"); // TODO: error handling
                        if let Some(code) = exit_status.code() {
                            let _ = child_exit_tx.send(JobStatus::Exited { code });
                        } else if let Some(signal) = exit_status.signal() {
                            let _ = child_exit_tx.send(JobStatus::Killed { signal });
                        } else {
                            unreachable!()
                        }
                        break; // exit select loop
                    }
                }
            }
        });

        // pipe stdout to the broadcaster
        if let Some(mut stdout) = maybe_stdout {
            let stdout_tx = broadcast_tx.clone();
            tokio::spawn(async move {
                let mut buf = BytesMut::with_capacity(4096);
                loop {
                    match stdout.read_buf(&mut buf).await {
                        Ok(n) if n > 0 => {
                            // move the bytes out of buf and into a message
                            let msg = Output::Stdout(buf.split().freeze());
                            let _ = stdout_tx.send(msg);
                        }
                        _ => {
                            break;
                        }
                    }
                }
            });
        }

        // pipe stderr to the broadcaster
        if let Some(mut stderr) = maybe_stderr {
            let stderr_tx = broadcast_tx;
            tokio::spawn(async move {
                let mut buf = BytesMut::with_capacity(4096);
                loop {
                    match stderr.read_buf(&mut buf).await {
                        Ok(n) if n > 0 => {
                            // move the bytes out of buf and into a message
                            let msg = Output::Stderr(buf.split().freeze());
                            let _ = stderr_tx.send(msg);
                        }
                        _ => {
                            break;
                        }
                    }
                }
            });
        }
        self.handle_messages(child_exit_rx).await;
    }

    async fn handle_messages(&mut self, child_exit_rx: oneshot::Receiver<JobStatus>) {
        use WorkerMessage::*;
        let mut child_exit_rx = child_exit_rx.fuse();
        loop {
            select! {
                maybe_msg = self.inbox.recv() => {
                    if let Some(msg) = maybe_msg {
                        match msg {
                            GetStatus { response } => {
                                let _ = response.send(Ok(self.job_status));
                            }
                            Stop { response } => {
                                match (self.job_status, self.kill_tx.take()) {
                                    (JobStatus::Running, Some(kill_tx)) => {
                                        let _ = kill_tx.send(());
                                        let _ = response.send(Ok(()));
                                    }
                                    _ =>  {
                                        let _ = response.send(Err(JobError::AlreadyStopped));
                                    }
                                }
                            }
                        }
                    } else {
                        // actor handle dropped, make sure we kill the child process before we exit
                        if let Some(kill_tx) = self.kill_tx.take() {
                            let _ = kill_tx.send(());
                        }
                        return;
                    }
                }
                exit_status = &mut child_exit_rx => {
                    let _ = exit_status.map(|exit_status| self.job_status = exit_status);
                }
            }
        }
    }
}
