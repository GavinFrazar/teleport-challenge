use std::collections::HashMap;

use protobuf::{
    output_request::OutputType, remote_jobs_client::RemoteJobsClient, status_response::JobStatus,
    OutputRequest, OutputResponse, StartRequest, StatusRequest,
};

use std::path::PathBuf;
use tonic::{
    transport::{Certificate, Channel, ClientTlsConfig, Identity},
    Request, Status,
};

type JobId = uuid::Uuid;

pub struct ClientCli {
    inner: RemoteJobsClient<Channel>,
}

impl ClientCli {
    pub async fn connect(user: &str, server_addr: &str) -> Self {
        let tls = build_tls_config(user).await;

        let channel = Channel::from_shared(format!("https://{}", server_addr))
            .expect("channel parse error")
            .tls_config(tls)
            .expect("tls config")
            .connect()
            .await
            .expect("channel connect");

        Self {
            inner: RemoteJobsClient::new(channel),
        }
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

async fn build_tls_config(user: &str) -> ClientTlsConfig {
    let server_root_ca_cert = include_bytes!("../../tls/data/server_ca.pem");
    let server_root_ca_cert = Certificate::from_pem(server_root_ca_cert);

    let mut pathbuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    pathbuf.push("..");
    pathbuf.push("tls");
    pathbuf.push("data");

    // get user cert path
    pathbuf.push(format!("{}.pem", user));
    let client_cert_path = pathbuf
        .canonicalize()
        .unwrap_or_else(|_| panic!("missing client cert: {:?}", pathbuf));
    pathbuf.pop();

    // get user key path
    pathbuf.push(format!("{}.key", user));
    let client_key_path = pathbuf
        .canonicalize()
        .unwrap_or_else(|_| panic!("missing client key: {:?}", pathbuf));

    // read client cert
    let client_cert = tokio::fs::read(client_cert_path.clone())
        .await
        .unwrap_or_else(|_| panic!("failed to read {:?}", client_cert_path));

    // read client key
    let client_key = tokio::fs::read(client_key_path.clone())
        .await
        .unwrap_or_else(|_| panic!("failed to read {:?}", client_key_path));
    let client_identity = Identity::from_pem(client_cert, client_key);

    ClientTlsConfig::new()
        .domain_name("localhost")
        .ca_certificate(server_root_ca_cert)
        .identity(client_identity)
}
