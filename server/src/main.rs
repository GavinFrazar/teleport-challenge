mod interceptors;
mod services;

pub use cert::UserExtension;
use interceptors::cert;
use protobuf::remote_jobs_server::RemoteJobsServer;
pub use services::jobservice::RemoteJobsService;
use tokio_rustls::rustls::{
    self, ciphersuite::TLS13_AES_256_GCM_SHA384, AllowAnyAuthenticatedClient, RootCertStore,
    ServerConfig,
};
use tonic::transport::{Server, ServerTlsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051";
    serve(addr).await
}

async fn serve(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let addr = addr.parse().unwrap();

    // load client CA cert
    let client_ca_der: &[u8] = include_bytes!("../../tls/data/client_ca.der");
    let client_ca_cert_der = rustls::Certificate(client_ca_der.into());
    let mut client_roots = RootCertStore::empty();
    client_roots
        .add(&client_ca_cert_der)
        .expect("error reading DER encoded ca cert");
    let client_auth = AllowAnyAuthenticatedClient::new(client_roots);
    let cipher_suites = &[&TLS13_AES_256_GCM_SHA384];
    let mut rustls_config = ServerConfig::with_ciphersuites(client_auth, cipher_suites);

    // load server certificate
    let server_der: &[u8] = include_bytes!("../../tls/data/server.der");
    let server_cert_chain = vec![rustls::Certificate(server_der.into())];

    // load server key
    let mut server_key: &[u8] = include_bytes!("../../tls/data/server.key");
    let server_key = match rustls_pemfile::read_one(&mut server_key)
        .expect("cannot parse server private key file")
    {
        Some(rustls_pemfile::Item::ECKey(key)) => rustls::PrivateKey(key),
        Some(rustls_pemfile::Item::PKCS8Key(key)) => rustls::PrivateKey(key),
        thing => panic!("No server key, got thing:\n {:?}", thing),
    };

    // use server cert/key
    rustls_config
        .set_single_cert(server_cert_chain, server_key)
        .expect("server cert parse err");

    // use HTTP/2 over tls
    rustls_config.set_protocols(&[b"h2".to_vec()]);

    // Create the tonic server config
    let tls_config = ServerTlsConfig::new()
        .rustls_server_config(rustls_config)
        .to_owned();
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
    use protobuf::status_response::JobStatus;
    use protobuf::{remote_jobs_client::RemoteJobsClient, StartRequest};
    use protobuf::{OutputRequest, OutputResponse, StatusRequest};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
    use tonic::Code;

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
    async fn user_authenticates() {
        let addr = "[::1]:50051";
        start_server(addr).await;
        let _client = build_client("alice", addr).await;
    }

    #[tokio::test]
    async fn user_cannot_authenticate() {
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
        assert!(response.is_err());
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
            job_id: job_id.clone(),
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

        // check for status
        let status = client
            .query_status(tonic::Request::new(StatusRequest { job_id }))
            .await
            .expect("no status response")
            .into_inner()
            .job_status
            .expect("got empty job status");
        match status {
            JobStatus::ExitCode(code) => {
                assert_eq!(code, 0)
            }
            status => panic!("unexpected job status: {:?}", status),
        }
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

    #[tokio::test]
    async fn handles_command_errors() {
        let addr = "[::1]:50055";
        start_server(addr).await;
        let mut client = build_client("charlie", addr).await;

        // request a job for a command that doesnt exist
        let request = tonic::Request::new(StartRequest {
            cmd: "foo_bar_asfd".into(),
            args: vec!["-n".into(), "hello charlie".into()],
            dir: "/tmp".into(),
            envs: HashMap::new(),
        });
        let response = client.start_job(request).await;
        match response {
            Err(err) if err.code() == Code::NotFound => {}
            Err(err) => panic!("Job failed for unexpected reason: {}", err),
            Ok(_) => panic!("Job succeeded even with empty PATH"),
        }

        // request a job for a file without +x permissions set
        let request = tonic::Request::new(StartRequest {
            cmd: "/etc/hosts".into(), // pretty sure bet this is on the machine and not executable
            args: vec![],
            dir: "/tmp".into(),
            envs: HashMap::new(),
        });
        let response = client.start_job(request).await;
        match response {
            Err(err) if err.code() == Code::PermissionDenied => {} // os permission denied, not our authz!
            Err(err) => panic!("Job failed for unexpected reason: {}", err),
            Ok(_) => panic!("Job succeeded even with empty PATH"),
        }
    }
}
