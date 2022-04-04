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
    use std::collections::HashMap;

    #[tokio::test]
    async fn authentication() {
        use protobuf::{remote_jobs_client::RemoteJobsClient, StartRequest};
        use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
        let server_root_ca_cert = include_bytes!("../../tls/ca.cert");
        let server_root_ca_cert = Certificate::from_pem(server_root_ca_cert);
        let client_cert = include_bytes!("../../tls/alice.cert");
        let client_key = include_bytes!("../../tls/alice.key");
        let client_identity = Identity::from_pem(client_cert, client_key);

        let tls = ClientTlsConfig::new()
            .domain_name("localhost")
            .ca_certificate(server_root_ca_cert)
            .identity(client_identity);

        let channel = Channel::from_static("http://[::1]:50051")
            .tls_config(tls)
            .expect("tls config")
            .connect()
            .await
            .expect("channel connect");

        let mut client = RemoteJobsClient::new(channel);

        let request = tonic::Request::new(StartRequest {
            cmd: "echo".into(),
            args: vec!["hello alice".into()],
            dir: "/tmp".into(),
            envs: HashMap::new(),
        });

        let response = client.start_job(request).await;

        println!("RESPONSE={:?}", response);
        assert!(response.is_ok());
    }
}
