use crate::types::OutputBlob;

#[derive(Clone, Copy, Debug)]
pub enum JobStatus {
    Idle,
    Running,
    Exited { code: i32 },
    Killed { signal: i32 },
}

#[derive(Clone)]
pub enum Output {
    Stdout(OutputBlob),
    Stderr(OutputBlob),
}
