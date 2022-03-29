use bytes::Bytes;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

mod actor;
mod messages;

/// A `Broadcaster` which can add subscribers, receive output, and broadcast the output to all subscribers.
///
/// This struct is actually an actor handle. The real work is done in the actor spawned by `Broadcaster::new`.
#[derive(Clone)]
pub struct Broadcaster {}

impl Broadcaster {
    pub fn new(output_rx: UnboundedReceiver<Bytes>) -> Self {
        todo!()
    }

    pub async fn subscribe_stdout(&self, output_tx: UnboundedSender<Bytes>) {
        todo!()
    }

    pub async fn subscribe_stderr(&self, output_tx: UnboundedSender<Bytes>) {
        todo!()
    }

    pub async fn subscribe_all_out(&self, output_tx: UnboundedSender<Bytes>) {
        todo!()
    }
}
