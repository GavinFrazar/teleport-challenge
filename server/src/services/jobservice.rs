mod authz;
use self::authz::{AuthzDb, Permission, Scope};
use crate::UserExtension;

use futures::Stream;
use joblib::{types::JobId, JobCoordinator};
use protobuf::{
    output_request::OutputType,
    remote_jobs_server::RemoteJobs,
    status_response::JobStatus::{ExitCode, KillSignal, Running},
    OutputRequest, OutputResponse, StartRequest, StartResponse, StatusRequest, StatusResponse,
    StopRequest, StopResponse,
};
use std::{collections::HashMap, pin::Pin, sync::Mutex};
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use tonic::{self, Request, Response, Status};
use uuid::Uuid;

pub type UserId = String;
type JobOwnerDb = HashMap<JobId, UserId>;

/// A job service for remote job start/stop/status/output api.
///
/// Jobs are assigned an owner when they start - the `user id` of the user who started the job.
///
/// Authorization is provided by a mock authz database interface
pub struct RemoteJobsService {
    coordinator: JobCoordinator,
    job_owners: Mutex<JobOwnerDb>, // tonic wraps the struct in Arc internally, so we don't need Arc
    authz_db: AuthzDb,             // immutable pre-populated mock db
}

impl Default for RemoteJobsService {
    fn default() -> Self {
        Self::new(1024)
    }
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

    fn is_authorized(&self, user_id: &UserId, action: Action) -> bool {
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
                            if job_owner == *user_id {
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
                            if job_owner == *user_id {
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
    type StreamOutputStream = Pin<Box<dyn Stream<Item = Result<OutputResponse, Status>> + Send>>;

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

        // check authz
        if !self.is_authorized(&user_id, Action::StartJob) {
            return Err(Status::permission_denied("Permission denied"));
        }

        let StartRequest {
            cmd,
            args,
            dir,
            envs,
        } = req.into_inner();

        let envs = Vec::from_iter(envs);
        let job_id = self
            .coordinator
            .start_job(cmd, args, dir, envs)
            .await
            .expect("failed to start job");

        self.job_owners
            .lock()
            .unwrap()
            .insert(job_id, user_id.to_string());
        Ok(Response::new(StartResponse {
            job_id: job_id.as_bytes().to_vec(),
        }))
    }

    async fn stop_job(&self, req: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        let user_id = req
            .extensions()
            .get::<UserExtension>()
            .unwrap()
            .user_id
            .clone();

        let job_id = req.into_inner().job_id;
        let job_id =
            Uuid::from_slice(&job_id).map_err(|err| Status::invalid_argument(err.to_string()))?;

        // check authz
        if !self.is_authorized(
            &user_id,
            Action::ExistingJob {
                job_id,
                inner_action: ExistingJobAction::StopJob,
            },
        ) {
            return Err(Status::permission_denied("Permission denied"));
        }

        self.coordinator
            .stop_job(job_id)
            .await
            .map_err(|err| match err {
                joblib::error::Error::AlreadyStopped => Status::internal(err.to_string()),
                joblib::error::Error::DoesNotExist => unreachable!(), // no job, so authz should have failed
            })?;
        Ok(Response::new(StopResponse {})) // empty response on success
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

        let job_id = req.into_inner().job_id;
        let job_id =
            Uuid::from_slice(&job_id).map_err(|err| Status::invalid_argument(err.to_string()))?;

        // check authz
        if !self.is_authorized(
            &user_id,
            Action::ExistingJob {
                job_id,
                inner_action: ExistingJobAction::QueryStatus,
            },
        ) {
            return Err(Status::permission_denied("Permission denied"));
        }

        let job_status = self
            .coordinator
            .get_job_status(job_id)
            .await
            .map_err(|err| Status::internal(err.to_string()))?;
        let status = match job_status {
            joblib::events::JobStatus::Running => Running(true),
            joblib::events::JobStatus::Exited { code } => ExitCode(code),
            joblib::events::JobStatus::Killed { signal } => KillSignal(signal),
        };
        let status_response = StatusResponse {
            job_status: Some(status),
        };
        Ok(Response::new(status_response))
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

        let job_id = &req.get_ref().job_id;
        let job_id =
            Uuid::from_slice(job_id).map_err(|err| Status::invalid_argument(err.to_string()))?;

        // check authz
        if !self.is_authorized(
            &user_id,
            Action::ExistingJob {
                job_id,
                inner_action: ExistingJobAction::StreamOutput,
            },
        ) {
            return Err(Status::permission_denied("Permission denied"));
        }

        let receiver_result = match req.into_inner().output() {
            OutputType::Stdout => self.coordinator.stream_stdout(job_id).await,
            OutputType::Stderr => self.coordinator.stream_stderr(job_id).await,
            OutputType::All => self.coordinator.stream_all(job_id).await,
        };
        let receiver = receiver_result.map_err(|err| Status::internal(err.to_string()))?;

        let output_stream = UnboundedReceiverStream::from(receiver);
        let response_stream = output_stream.map(|blob| {
            Ok(OutputResponse {
                data: blob.to_vec(),
            })
        });
        Ok(Response::new(
            Box::pin(response_stream) as Self::StreamOutputStream
        ))
    }
}
