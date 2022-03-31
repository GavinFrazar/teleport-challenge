mod actor;
mod messages;

use crate::error;
use crate::events::JobStatus;
use crate::types::{Args, Dir, Envs, Output, Program};
use actor::Actor;
use messages::WorkerMessage;
use std::{io, process::Stdio};
use tokio::{
    process,
    sync::{mpsc, oneshot},
};

#[derive(Clone)]
pub struct WorkerHandle {
    sender: mpsc::UnboundedSender<WorkerMessage>,
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
        let (sender, inbox) = mpsc::unbounded_channel();
        Actor::spawn(inbox, output_tx, child);
        Ok(Self { sender })
    }

    pub fn get_status(&self, status_tx: oneshot::Sender<error::Result<JobStatus>>) {
        let _ = self.sender.send(WorkerMessage::GetStatus {
            response: status_tx,
        });
    }

    pub fn stop(&self, response: oneshot::Sender<error::Result<()>>) {
        let _ = self.sender.send(WorkerMessage::Stop { response });
    }
}
