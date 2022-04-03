use joblib::types::JobId;
use joblib::JobCoordinator;
use protobuf::remote_jobs_server::RemoteJobs;
use protobuf::{
    OutputRequest, OutputResponse, StartRequest, StartResponse, StatusRequest, StatusResponse,
    StopRequest, StopResponse,
};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio;
use tokio_stream::wrappers::ReceiverStream;
use tonic;
use tonic::{Request, Response, Status};

use crate::UserExtension;

type UserId = bytes::Bytes;
type Roles = HashSet<Role>;
type Permissions = HashMap<Scope, Roles>;
type JobOwnerDb = HashMap<JobId, UserId>;
type AuthzDb = HashMap<UserId, Permissions>;

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
enum Role {
    TaskManager,
    Analyst,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
enum Scope {
    Owner,
    All,
}

#[derive(Default)]
// tonic wraps this in Arc anyway internally, so we don't need Arc
pub struct RemoteJobsService {
    coordinator: JobCoordinator,
    job_owners: Mutex<JobOwnerDb>,
    authz_db: AuthzDb, // immutable pre-populated mock db
}

impl RemoteJobsService {
    pub fn new(channel_capacity: usize) -> Self {
        // Construct the mock db
        let mut mock_db = AuthzDb::new();

        // give "alice" permission to start jobs and stop jobs she owns
        let mut alice_permissions = HashMap::new();
        alice_permissions.insert(Scope::Owner, HashSet::from_iter(vec![Role::TaskManager]));
        mock_db.insert("alice".into(), alice_permissions);

        let mut bob_permissions = HashMap::new();
        bob_permissions.insert(Scope::All, HashSet::from_iter(vec![Role::Analyst]));
        mock_db.insert("bob".into(), bob_permissions);

        let mut charlie_permissions = HashMap::new();
        charlie_permissions.insert(Scope::All, HashSet::from_iter(vec![Role::TaskManager]));
        mock_db.insert("charlie".into(), charlie_permissions);

        Self {
            coordinator: JobCoordinator::spawn(channel_capacity),
            job_owners: Mutex::new(JobOwnerDb::new()),
            authz_db: mock_db,
        }
    }

    async fn is_authorized(&self, user_id: UserId, action: Action) -> bool {
        use Action::*;
        if let Some(permissions) = self.authz_db.get(&user_id) {
            let job_owner = match action {
                StopJob { job_id } | QueryStatus { job_id } | StreamOutput { job_id } => {
                    self.job_owners.lock().unwrap().get(&job_id).cloned()
                }
                _ => None,
            };
        }
        false
    }
}

enum Action {
    StartJob,
    StopJob { job_id: JobId },
    QueryStatus { job_id: JobId },
    StreamOutput { job_id: JobId },
}

#[tonic::async_trait]
impl RemoteJobs for RemoteJobsService {
    type StreamOutputStream = ReceiverStream<Result<OutputResponse, Status>>;

    async fn start_job(
        &self,
        req: Request<StartRequest>,
    ) -> Result<Response<StartResponse>, Status> {
        let user_id = req
            .extensions()
            .get::<UserExtension>()
            .unwrap()
            .user_id
            .clone();
        println!(
            "Starting job for user: {}",
            String::from_utf8_lossy(&user_id)
        );
        let StartRequest {
            cmd,
            args,
            dir,
            envs,
        } = req.into_inner();
        let envs = vec![]; // TODO: envs is hashmap but coordinator takes vec
        let job_id = self
            .coordinator
            .start_job(cmd, args, dir, envs)
            .await
            .expect("failed to start job");
        Ok(Response::new(StartResponse {
            id: job_id.as_bytes().to_vec(),
        }))
    }

    async fn stop_job(&self, req: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        let user_id = req
            .extensions()
            .get::<UserExtension>()
            .unwrap()
            .user_id
            .clone();
        todo!()
    }

    async fn query_status(
        &self,
        req: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        let user_id = req
            .extensions()
            .get::<UserExtension>()
            .unwrap()
            .user_id
            .clone();
        todo!()
    }

    async fn stream_output(
        &self,
        req: Request<OutputRequest>,
    ) -> Result<Response<Self::StreamOutputStream>, Status> {
        let user_id = req
            .extensions()
            .get::<UserExtension>()
            .unwrap()
            .user_id
            .clone();
        todo!()
    }
}
