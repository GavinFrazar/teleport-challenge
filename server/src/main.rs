#![allow(dead_code, unused_imports, unused_variables)]

mod interceptors;
mod services;

pub use cert::UserExtension;
use interceptors::cert;
use protobuf::remote_jobs_server::RemoteJobsServer;
use services::jobservice::RemoteJobsService;
use tonic::{
    transport::{
        server::{TcpConnectInfo, TlsConnectInfo},
        Certificate, Identity, Server, ServerTlsConfig,
    },
    Request, Status,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051";
    serve(addr).await
}

async fn serve(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let addr = addr.parse().unwrap();

    // load server identity
    let cert = include_bytes!("../../tls/server.cert");
    let key = include_bytes!("../../tls/server.key");
    let server_identity = Identity::from_pem(cert, key);

    // load CA cert
    let ca_cert = include_bytes!("../../tls/ca.cert");
    let ca_cert = Certificate::from_pem(ca_cert);

    let tls_config = ServerTlsConfig::new()
        .identity(server_identity)
        .client_ca_root(ca_cert);

    let job_service = RemoteJobsService::default();
    let remote_jobs_server =
        RemoteJobsServer::with_interceptor(job_service, cert::extract_subj_uid);
    println!("Listening on {}", addr);

    Server::builder()
        .tls_config(tls_config)?
        .add_service(remote_jobs_server)
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use protobuf::output_request::OutputType;
    use protobuf::{remote_jobs_client::RemoteJobsClient, StartRequest};
    use protobuf::{OutputRequest, OutputResponse};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};

    // start the server
    async fn start_server(addr: &'static str) {
        tokio::spawn(async move {
            let _ = serve(addr).await;
        });
        // wait a short duration so server can start before clients connect
        // TODO: do something more robust to wait for server start
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    async fn build_tls_config(user: &str) -> ClientTlsConfig {
        let server_root_ca_cert = include_bytes!("../../tls/ca.cert");
        let server_root_ca_cert = Certificate::from_pem(server_root_ca_cert);

        let mut pathbuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        pathbuf.push("..");
        pathbuf.push("tls");

        // get user cert path
        pathbuf.push(format!("{}.cert", user));
        let client_cert_path = pathbuf
            .canonicalize()
            .expect(&format!("missing client cert: {:?}", pathbuf));
        pathbuf.pop();

        // get user key path
        pathbuf.push(format!("{}.key", user));
        let client_key_path = pathbuf
            .canonicalize()
            .expect(&format!("missing client key: {:?}", pathbuf));

        // read client cert
        let client_cert = tokio::fs::read(client_cert_path.clone())
            .await
            .expect(&format!("failed to read {:?}", client_cert_path));

        // read client key
        let client_key = tokio::fs::read(client_key_path.clone())
            .await
            .expect(&format!("failed to read {:?}", client_key_path));
        let client_identity = Identity::from_pem(client_cert, client_key);

        ClientTlsConfig::new()
            .domain_name("localhost")
            .ca_certificate(server_root_ca_cert)
            .identity(client_identity)
    }

    async fn build_client(
        user: &'static str,
        server_addr: &'static str,
    ) -> RemoteJobsClient<Channel> {
        let tls = build_tls_config(user).await;

        let channel = Channel::from_shared(format!("https://{}", server_addr))
            .expect("channel parse error")
            .tls_config(tls)
            .expect("tls config")
            .connect()
            .await
            .expect("channel connect");

        RemoteJobsClient::new(channel)
    }

    #[tokio::test]
    async fn authenticated_user() {
        let addr = "[::1]:50051";
        start_server(addr).await;
        let client = build_client("alice", addr).await;
    }

    #[tokio::test]
    async fn unauthenticated_user() {
        let addr = "[::1]:50052";
        start_server(addr).await;
        let mut client = build_client("eve", addr).await;
        let request = tonic::Request::new(StartRequest {
            cmd: "echo".into(),
            args: vec!["hello eve".into()],
            dir: "/tmp".into(),
            envs: HashMap::new(),
        });
        let response = client.start_job(request).await;
        match response {
            Err(status) => match status.code() {
                tonic::Code::Unauthenticated => {}
                code => {
                    panic!(
                        "unauthenticated user got Err status, but for unexpected reason {}",
                        code
                    );
                }
            },
            _ => {
                panic!("unauthenticated user got Ok response!")
            }
        }
    }

    #[tokio::test]
    async fn authorized_user() {
        let addr = "[::1]:50053";
        start_server(addr).await;
        let mut client = build_client("alice", addr).await;

        // start an echo job
        let request = tonic::Request::new(StartRequest {
            cmd: "echo".into(),
            args: vec!["-n".into(), "hello alice".into()],
            dir: "/tmp".into(),
            envs: HashMap::new(),
        });
        let response = client
            .start_job(request)
            .await
            .expect("Bad start job response");
        let job_id = response.into_inner().job_id;

        // get the output
        let stream_request = tonic::Request::new(OutputRequest {
            job_id,
            output: OutputType::All.into(),
        });
        let mut stream = client
            .stream_output(stream_request)
            .await
            .expect("no stream response")
            .into_inner();
        let mut received = vec![];
        while let Some(OutputResponse { data }) = stream.message().await.unwrap() {
            received.extend_from_slice(&data);
        }
        assert_eq!("hello alice", String::from_utf8_lossy(&received));
    }

    #[tokio::test]
    async fn unauthorized_user() {
        let addr = "[::1]:50054";
        start_server(addr).await;
        let mut client = build_client("bob", addr).await;

        let request = tonic::Request::new(StartRequest {
            cmd: "echo".into(),
            args: vec!["hello bob".into()],
            dir: "/tmp".into(),
            envs: HashMap::new(),
        });
        let response = client.start_job(request).await;
        match response {
            Err(status) => match status.code() {
                tonic::Code::PermissionDenied => {}
                code => {
                    panic!(
                        "unauthorized user got Err status, but for unexpected reason {}",
                        code
                    );
                }
            },
            _ => {
                panic!("unauthorized user got Ok response!")
            }
        }
    }
}
