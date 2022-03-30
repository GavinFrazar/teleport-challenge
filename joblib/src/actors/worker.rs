mod actor;
mod messages;

use crate::events::{JobStatus, Output};
use crate::types::{Args, Dir, Envs, Program};
use actor::Actor;
use messages::WorkerMessage;
use std::{io, process::Stdio};
use tokio::{
    process,
    sync::{mpsc, oneshot},
};

#[derive(Clone)]
pub struct WorkerHandle {
    sender: mpsc::Sender<WorkerMessage>,
}

impl WorkerHandle {
    pub fn spawn(
        output_tx: mpsc::UnboundedSender<Output>,
        cmd: Program,
        args: Args,
        dir: Dir,
        envs: Envs,
    ) -> io::Result<Self> {
        let mut command = process::Command::new(cmd);
        let child = command
            .args(args)
            .current_dir(dir)
            .envs(envs)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let (sender, inbox) = mpsc::channel(8);
        Actor::spawn(inbox, output_tx, child);
        Ok(Self { sender })
    }

    pub async fn get_status(&self) -> JobStatus {
        let (status_tx, status_rx) = oneshot::channel();
        let _ = self
            .sender
            .send(WorkerMessage::GetStatus {
                response: status_tx,
            })
            .await;
        status_rx.await.expect("Worker exited")
    }

    pub async fn stop(&self) {
        let _ = self.sender.send(WorkerMessage::Stop).await;
    }
}
