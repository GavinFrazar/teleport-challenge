mod authz;
use self::authz::{AuthzDb, Permission, Scope};

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

pub type UserId = bytes::Bytes;
type JobOwnerDb = HashMap<JobId, UserId>;

// tonic wraps this in Arc anyway internally, so we don't need Arc
#[derive(Default)]
pub struct RemoteJobsService {
    coordinator: JobCoordinator,
    job_owners: Mutex<JobOwnerDb>,
    authz_db: AuthzDb, // immutable pre-populated mock db
}

enum ExistingJobAction {
    StopJob,
    QueryStatus,
    StreamOutput,
}

enum Action {
    StartJob,
    ExistingJob {
        job_id: JobId,
        inner_action: ExistingJobAction,
    },
}

impl RemoteJobsService {
    pub fn new(channel_capacity: usize) -> Self {
        let authz_db = AuthzDb::default();
        Self {
            coordinator: JobCoordinator::spawn(channel_capacity),
            job_owners: Mutex::new(JobOwnerDb::new()),
            authz_db,
        }
    }

    async fn is_authorized(&self, user_id: UserId, action: Action) -> bool {
        use Action::*;
        use ExistingJobAction::*;
        match action {
            ExistingJob {
                job_id,
                inner_action,
            } => {
                let maybe_owner = self.job_owners.lock().unwrap().get(&job_id).cloned();
                if let Some(job_owner) = maybe_owner {
                    match inner_action {
                        StopJob => {
                            if job_owner == user_id {
                                return self
                                    .authz_db
                                    .has_permission(user_id, Permission::StartOrStop);
                            } else {
                                return self.authz_db.has_scoped_permission(
                                    user_id,
                                    Scope::All,
                                    Permission::StartOrStop,
                                );
                            }
                        }
                        QueryStatus | StreamOutput => {
                            if job_owner == user_id {
                                return self.authz_db.has_permission(user_id, Permission::Query);
                            } else {
                                return self.authz_db.has_scoped_permission(
                                    user_id,
                                    Scope::All,
                                    Permission::Query,
                                );
                            }
                        }
                    }
                }
            }
            StartJob => {
                return self
                    .authz_db
                    .has_permission(user_id, Permission::StartOrStop)
            }
        }

        // reject anything else as unauthorized
        // NOTE: if the user id doesnt exist, or the job id doesnt exist, we reject those as unauth --
        //       -- dont leak info! Although I won't go as far as hardening this against timing attacks.
        false
    }
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
