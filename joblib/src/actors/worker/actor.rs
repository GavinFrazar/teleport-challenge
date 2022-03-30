use super::messages::WorkerMessage;
use crate::events::{JobStatus, Output};

use bytes::{Bytes, BytesMut};
use futures::future::FutureExt;
use std::os::unix::process::ExitStatusExt;
use tokio::{
    io::AsyncReadExt,
    process::Child,
    select,
    sync::{mpsc, oneshot},
};

pub struct Actor {
    inbox: mpsc::Receiver<WorkerMessage>,
    kill_tx: Option<oneshot::Sender<()>>,
    job_status: JobStatus,
}

impl Actor {
    pub fn spawn(
        inbox: mpsc::Receiver<WorkerMessage>,
        output_tx: mpsc::UnboundedSender<Output>,
        child: Child,
    ) {
        let (kill_tx, kill_rx) = oneshot::channel();
        let actor = Self {
            inbox,
            kill_tx: Some(kill_tx),
            job_status: JobStatus::Running,
        };
        tokio::spawn(async move { actor.run(output_tx, kill_rx, child) });
    }

    pub async fn run(
        mut self,
        output_tx: mpsc::UnboundedSender<Output>,
        mut kill_rx: oneshot::Receiver<()>,
        mut child: tokio::process::Child,
    ) {
        let (child_exit_tx, child_exit_rx) = oneshot::channel();
        let mut stdout = child.stdout.take().expect("child stdout not piped"); //TODO: error handling
        let mut stderr = child.stderr.take().expect("child stderr not piped"); // could just optionally pipe
        tokio::spawn(async move {
            loop {
                select! {
                    // listen for a kill signal
                    _ = &mut kill_rx => {
                        let _ = child.start_kill();
                    }
                    // wait for child pid to finish and cleanup its resources
                    exit_status = child.wait() => {
                        println!("child finished"); // TODO: cleanup debugging print
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

        let stdout_tx = output_tx.clone();
        tokio::spawn(async move {
            let mut buf = BytesMut::with_capacity(4096);
            loop {
                let read_result = stdout.read_buf(&mut buf).await;
                match read_result {
                    Ok(n) if n > 0 => {
                        let output = Output::Stdout(Bytes::copy_from_slice(&buf[..n]));
                        let _ = stdout_tx.send(output);
                    }
                    _ => {
                        println!("child stdout finished!"); // TODO: remove debug prints
                        break;
                    }
                }
            }
        });

        let stderr_tx = output_tx;
        tokio::spawn(async move {
            let mut buf = BytesMut::with_capacity(4096);
            loop {
                let read_result = stderr.read_buf(&mut buf).await;
                match read_result {
                    Ok(n) if n > 0 => {
                        let output = Output::Stdout(Bytes::copy_from_slice(&buf[..n]));
                        let _ = stderr_tx.send(output);
                    }
                    _ => {
                        println!("child stderr finished!"); // TODO: remove debug prints
                        break;
                    }
                }
            }
        });

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
                                let _ = response.send(self.job_status);
                            }
                            Stop => {
                                self.kill_tx.take().map(|kill_tx| kill_tx.send(()));
                            }
                        }
                    } else {
                        // actor handle dropped, make sure we kill the child process before we exit
                        self.kill_tx.take().map(|kill_tx| kill_tx.send(()));
                        break;
                    }
                }
                exit_status = &mut child_exit_rx => {
                    let _ = exit_status.map(|exit_status| self.job_status = exit_status);
                }
            }
        }
    }
}
