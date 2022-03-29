use crate::events::JobStatus;

mod actor;
mod messages;

#[derive(Clone)]
pub struct Worker {}

impl Worker {
    pub fn spawn(cmd: String, args: Vec<String>) -> Self {
        todo!()
    }

    pub async fn get_status(&self) -> JobStatus {
        todo!()
    }

    pub async fn kill(&self) {
        todo!()
    }
}
