use super::messages::StreamRequest;
use crate::events::OutputBlob;
use crate::types::Output;

use tokio::select;
use tokio::sync::mpsc;

pub struct Actor {
    inbox: mpsc::UnboundedReceiver<StreamRequest>,
    output_rx: mpsc::UnboundedReceiver<Output>,
    output_buffer: Vec<Output>,
    stdout_subscribers: Vec<mpsc::UnboundedSender<OutputBlob>>,
    stderr_subscribers: Vec<mpsc::UnboundedSender<OutputBlob>>,
    output_pending: bool,
}

impl Actor {
    pub fn spawn(
        inbox: mpsc::UnboundedReceiver<StreamRequest>,
        output_rx: mpsc::UnboundedReceiver<Output>,
    ) {
        let actor = Actor {
            inbox,
            output_rx,
            output_buffer: Vec::new(),
            stdout_subscribers: Vec::new(),
            stderr_subscribers: Vec::new(),
            output_pending: true,
        };
        tokio::spawn(async move { actor.run().await });
    }

    async fn run(mut self) {
        use self::StreamRequest::*;
        loop {
            select! {
                maybe_stream_req = self.inbox.recv() => {
                    if let Some(req) = maybe_stream_req {
                        match req {
                            Stdout { subscriber } => self.stream_stdout(subscriber),
                            Stderr { subscriber  } => self.stream_stderr(subscriber),
                            All { subscriber } => self.stream_all(subscriber),
                        }
                    } else {
                        // actor handle dropped, make sure we drop our senders before exiting
                        self.stdout_subscribers.clear();
                        self.stderr_subscribers.clear();
                        self.output_pending = false;
                        return;
                    }
                }
                maybe_output = self.output_rx.recv(), if self.output_pending => {
                    match maybe_output {
                        Some(output) => {
                            self.broadcast(output);
                        }
                        None => {
                            self.stdout_subscribers.clear();
                            self.stderr_subscribers.clear();
                            self.output_pending = false;
                        }
                    }
                }
            }
        }
    }

    fn broadcast(&mut self, output: Output) {
        self.output_buffer.push(output.clone());
        use self::Output::*;
        match output {
            Stdout(blob) => {
                // only retain subscribers who have not dropped
                self.stdout_subscribers
                    .retain(|sub| sub.send(blob.clone()).is_ok());
            }
            Stderr(blob) => {
                // only retain subscribers who have not dropped
                self.stderr_subscribers
                    .retain(|sub| sub.send(blob.clone()).is_ok());
            }
        }
    }

    fn stream_stdout(&mut self, output_tx: mpsc::UnboundedSender<OutputBlob>) {
        for blob in self
            .output_buffer
            .iter()
            .filter_map(|output| match output {
                Output::Stdout(blob) => Some(blob),
                _ => None,
            })
            .cloned()
        {
            if output_tx.send(blob).is_err() {
                // if receiver drops, that's fine, just ignore the error and stop sending
                // skip adding the subscriber too
                return;
            }
        }
        if self.output_pending {
            self.stdout_subscribers.push(output_tx);
        }
    }

    fn stream_stderr(&mut self, output_tx: mpsc::UnboundedSender<OutputBlob>) {
        for blob in self
            .output_buffer
            .iter()
            .filter_map(|output| match output {
                Output::Stderr(blob) => Some(blob),
                _ => None,
            })
            .cloned()
        {
            if output_tx.send(blob).is_err() {
                // if receiver drops, that's fine, just ignore the error and stop sending
                // skip adding the subscriber too
                return;
            }
        }
        if self.output_pending {
            self.stderr_subscribers.push(output_tx);
        }
    }

    fn stream_all(&mut self, output_tx: mpsc::UnboundedSender<OutputBlob>) {
        for blob in self
            .output_buffer
            .iter()
            .map(|output| match output {
                Output::Stdout(blob) | Output::Stderr(blob) => blob,
            })
            .cloned()
        {
            if output_tx.send(blob).is_err() {
                // if receiver drops, that's fine, just ignore the error and stop sending
                // skip adding the subscriber too
                return;
            }
        }
        if self.output_pending {
            self.stdout_subscribers.push(output_tx.clone());
            self.stderr_subscribers.push(output_tx);
        }
    }
}
