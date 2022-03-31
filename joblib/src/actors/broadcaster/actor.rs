use super::messages::Message;
use crate::events::Output;
use crate::types::OutputBlob;

use tokio::select;
use tokio::sync::mpsc;

pub struct Actor {
    inbox: mpsc::UnboundedReceiver<Message>,
    output_rx: mpsc::UnboundedReceiver<Output>,
    output_buffer: Vec<Output>,
    stdout_subscribers: Vec<mpsc::UnboundedSender<OutputBlob>>,
    stderr_subscribers: Vec<mpsc::UnboundedSender<OutputBlob>>,
    output_pending: bool,
}

impl Actor {
    pub fn spawn(
        inbox: mpsc::UnboundedReceiver<Message>,
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
        loop {
            select! {
                Some(msg) = self.inbox.recv() => {
                    use self::Message::*;
                    match msg {
                        StreamStdout { subscriber } => self.stream_stdout(subscriber),
                        StreamStderr { subscriber  } => self.stream_stderr(subscriber),
                        StreamAll { subscriber } => self.stream_all(subscriber),
                    }
                }
                maybe_output = self.output_rx.recv(), if self.output_pending => {
                    match maybe_output {
                        Some(output) => {
                            self.output_buffer.push(output.clone());
                            use self::Output::*;
                            match output {
                                Stdout(blob) => {
                                    self.stdout_subscribers.retain(|sub| {
                                        // only retain subscribers who have not dropped
                                        sub.send(blob.clone()).is_ok()
                                    });
                                }
                                Stderr(blob) => {
                                    self.stderr_subscribers.retain(|sub| {
                                        // only retain subscribers who have not dropped
                                        sub.send(blob.clone()).is_ok()
                                    });
                                }
                            }
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

    fn stream_stdout(&mut self, output_tx: mpsc::UnboundedSender<OutputBlob>) {
        self.output_buffer
            .iter()
            .filter_map(|output| match output {
                Output::Stdout(blob) => Some(blob),
                _ => None,
            })
            .cloned()
            .for_each(|blob| output_tx.send(blob).expect("stdout receiver dropped"));
        if self.output_pending {
            self.stdout_subscribers.push(output_tx);
        }
    }

    fn stream_stderr(&mut self, output_tx: mpsc::UnboundedSender<OutputBlob>) {
        self.output_buffer
            .iter()
            .filter_map(|output| match output {
                Output::Stderr(blob) => Some(blob),
                _ => None,
            })
            .for_each(|blob| {
                output_tx
                    .send(blob.clone())
                    .expect("stderr receiver dropped")
            });
        if self.output_pending {
            self.stderr_subscribers.push(output_tx);
        }
    }

    fn stream_all(&mut self, output_tx: mpsc::UnboundedSender<OutputBlob>) {
        self.output_buffer
            .iter()
            .map(|output| match output {
                Output::Stdout(blob) | Output::Stderr(blob) => blob,
            })
            .for_each(|blob| {
                output_tx
                    .send(blob.clone())
                    .expect("allout receiver dropped")
            });
        if self.output_pending {
            self.stdout_subscribers.push(output_tx.clone());
            self.stderr_subscribers.push(output_tx);
        }
    }
}
