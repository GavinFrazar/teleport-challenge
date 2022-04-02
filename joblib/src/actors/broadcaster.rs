mod actor;
mod messages;
use crate::{events::OutputBlob, types::Output};
use actor::Actor;
use messages::StreamRequest;

use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

/// A `Broadcaster` which can add subscribers, receive output, and broadcast the output to all subscribers.
///
/// This struct is actually an actor handle. The real work is done in the actor spawned by `Broadcaster::new`.
#[derive(Clone)]
pub struct BroadcasterHandle {
    sender: mpsc::UnboundedSender<StreamRequest>,
}

impl BroadcasterHandle {
    pub fn spawn(output_rx: UnboundedReceiver<Output>) -> Self {
        let (sender, inbox) = mpsc::unbounded_channel();
        Actor::spawn(inbox, output_rx);
        Self { sender }
    }

    pub fn stream_stdout(&self, subscriber: UnboundedSender<OutputBlob>) {
        let _ = self.sender.send(StreamRequest::Stdout { subscriber });
    }

    pub fn stream_stderr(&self, subscriber: UnboundedSender<OutputBlob>) {
        let _ = self.sender.send(StreamRequest::Stderr { subscriber });
    }

    pub fn stream_all(&self, subscriber: UnboundedSender<OutputBlob>) {
        let _ = self.sender.send(StreamRequest::All { subscriber });
    }
}
