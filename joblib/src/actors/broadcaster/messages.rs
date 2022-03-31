use crate::types::OutputBlob;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum Message {
    StreamStdout {
        subscriber: mpsc::UnboundedSender<OutputBlob>,
    },
    StreamStderr {
        subscriber: mpsc::UnboundedSender<OutputBlob>,
    },
    StreamAll {
        subscriber: mpsc::UnboundedSender<OutputBlob>,
    },
}
