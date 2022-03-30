use std::result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JobError {
    #[error("")]
    NotFound,
    #[error("")]
    NotRunning,
}

pub type Result<T> = result::Result<T, JobError>;
