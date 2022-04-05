use std::collections::HashMap;

use protobuf::{
    output_request::{self, OutputType},
    remote_jobs_client::RemoteJobsClient,
    status_response::JobStatus,
    OutputRequest, OutputResponse, StartRequest, StatusRequest,
};
use tonic::{transport::Channel, Request, Status};
use uuid::Uuid;

type JobId = Vec<u8>;

pub struct ClientCli {
    inner: RemoteJobsClient<Channel>,
}

impl ClientCli {
    pub async fn connect(user: &str, server_addr: &str) -> Self {
        todo!()
    }

    pub async fn start_job(
        &mut self,
        cmd: &str,
        args: &[String],
        dir: &str,
        envs: &[(String, String)],
    ) -> Result<(), Status> {
        // start an echo job
        let request = tonic::Request::new(StartRequest {
            cmd: cmd.into(),
            args: args.into(),
            dir: dir.into(),
            envs: HashMap::from_iter(envs.iter().cloned()),
        });
        let response = self.inner.start_job(request).await?;
        let job_id = response.into_inner().job_id;
        let uuid = Uuid::from_slice(&job_id).expect("server responded with invalid uuid");
        println!("Started job id: {}", uuid);
        Ok(())
    }

    pub async fn stop_job(&mut self, job_id: JobId) -> Result<(), Status> {
        let uuid =
            Uuid::from_slice(&job_id).expect("client cli: stop job called with invalid job uuid");
        let request = Request::new(protobuf::StopRequest {
            job_id: job_id.into(),
        });
        let _ = self.inner.stop_job(request).await?;
        println!("Stopped job id: {}", uuid);
        Ok(())
    }

    pub async fn query_status(&mut self, job_id: JobId) -> Result<(), Status> {
        let response = self
            .inner
            .query_status(tonic::Request::new(StatusRequest { job_id }))
            .await?;
        let status = response
            .into_inner()
            .job_status
            .expect("server responded with empty job status");
        match status {
            JobStatus::Running(_) => println!("Running"),
            JobStatus::ExitCode(code) => println!("Exited with code: {}", code),
            JobStatus::KillSignal(signal) => println!("Killed by signal: {}", signal),
        }
        Ok(())
    }

    /// convenience function
    pub async fn stream_stdout(&mut self, job_id: JobId) -> Result<(), Status> {
        let output_request = OutputRequest {
            job_id: job_id.into(),
            output: OutputType::Stdout.into(),
        };
        self.stream_output(output_request).await
    }

    /// convenience function
    pub async fn stream_stderr(&mut self, job_id: JobId) -> Result<(), Status> {
        let output_request = OutputRequest {
            job_id: job_id.into(),
            output: OutputType::Stderr.into(),
        };
        self.stream_output(output_request).await
    }

    /// convenience function
    pub async fn stream_all(&mut self, job_id: JobId) -> Result<(), Status> {
        let output_request = OutputRequest {
            job_id: job_id.into(),
            output: OutputType::All.into(),
        };
        self.stream_output(output_request).await
    }

    /// Stream the requested output
    async fn stream_output(&mut self, output_request: OutputRequest) -> Result<(), Status> {
        let request = Request::new(output_request);
        let response = self.inner.stream_output(request).await?;
        let mut stream = response.into_inner();
        while let Some(OutputResponse { data }) = stream.message().await? {
            print!("{}", String::from_utf8_lossy(&data));
        }
        Ok(())
    }
}
