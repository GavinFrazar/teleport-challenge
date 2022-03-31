use crate::events::OutputBlob;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum StreamRequest {
    Stdout {
        subscriber: mpsc::UnboundedSender<OutputBlob>,
    },
    Stderr {
        subscriber: mpsc::UnboundedSender<OutputBlob>,
    },
    All {
        subscriber: mpsc::UnboundedSender<OutputBlob>,
    },
}
