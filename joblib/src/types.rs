use crate::events::OutputBlob;
use uuid::Uuid;

// TODO: make these more generic. requiring exact types is too strict.
/// name of program
pub type Program = String;
/// args list
pub type Args = Vec<String>;
/// working directory for program
pub type Dir = String;
/// env vars for program
pub type Envs = Vec<(String, String)>;
/// job id used to track and manage jobs
pub type JobId = Uuid;

/// Output blobs distinguished by source of the output.
#[derive(Clone)]
pub enum Output {
    Stdout(OutputBlob),
    Stderr(OutputBlob),
}
