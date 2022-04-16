mod authorizer;

use self::authorizer::{Action, Authorizer, ExistingJobAction};
use crate::UserExtension;

use futures::Stream;
use joblib::JobCoordinator;
use protobuf::{
    output_request::OutputType,
    remote_jobs_server::RemoteJobs,
    status_response::JobStatus::{ExitCode, KillSignal, Running},
    OutputRequest, OutputResponse, StartRequest, StartResponse, StatusRequest, StatusResponse,
    StopRequest, StopResponse,
};
use std::pin::Pin;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use tonic::{self, Request, Response, Status};
use uuid::Uuid;

pub type UserId = String;

/// A job service for remote job start/stop/status/output api.
///
/// Jobs are assigned an owner when they start - the `user id` of the user who started the job.
///
/// Authorization is provided by a mock authz database interface
///
/// TODO: in real implementation, use middleware for authorization*
/// * Authorization is checked inside each procedure instead of doing it with middleware to reduce code size.
///   Unfortunately tonic's `interceptor` design only intercepts in-bound requests, but we need to intercept
///   the responses as well.
pub struct RemoteJobsService {
    coordinator: JobCoordinator,
    authorizer: Authorizer, // tonic wraps the struct in Arc internally, so we don't need Arc
}

impl Default for RemoteJobsService {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl RemoteJobsService {
    pub fn new(channel_capacity: usize) -> Self {
        Self {
            authorizer: Authorizer::new(),
            coordinator: JobCoordinator::spawn(channel_capacity),
        }
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
        if !self.authorizer.is_authorized(&user_id, Action::StartJob) {
            return Err(Status::permission_denied("Permission denied"));
        }

        let StartRequest {
            cmd,
            args,
            dir,
            envs,
        } = req.into_inner();

        let envs = Vec::from_iter(envs);
        let job_id = self.coordinator.start_job(cmd, args, dir, envs).await?;

        self.authorizer.add_job(job_id, &user_id);
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
        if !self.authorizer.is_authorized(
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
        if !self.authorizer.is_authorized(
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
        if !self.authorizer.is_authorized(
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
