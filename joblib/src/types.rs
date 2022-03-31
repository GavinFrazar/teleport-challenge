use uuid::Uuid;

// TODO: make these more generic. requiring exact types is too strict.
pub type Program = String;
pub type Args = Vec<String>;
pub type Dir = String;
pub type Envs = Vec<(String,String)>;
pub type JobId = Uuid;
pub type OutputBlob = bytes::Bytes;
