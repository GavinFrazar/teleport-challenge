use std::result;
use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No such job exists")]
    DoesNotExist,
    #[error("Job already stopped")]
    AlreadyStopped,
}

pub type Result<T> = result::Result<T, Error>;
