#[derive(Clone, Copy, Debug)]
pub enum JobStatus {
    Running,
    Exited { code: i32 },
    Killed { signal: i32 },
}

pub type OutputBlob = bytes::Bytes;
