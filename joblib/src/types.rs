use uuid::Uuid;

pub type Program = String;
pub type Args = Vec<String>;
pub type Dir = String;
pub type Envs = Vec<(String,String)>;
pub type JobId = Uuid;
pub type OutputBlob = bytes::Bytes;
