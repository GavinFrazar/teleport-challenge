#![allow(dead_code, unused_imports, unused_variables)]

mod services;
use protobuf::remote_jobs_server::RemoteJobsServer;
use services::jobservice::RemoteJobsService;
use tonic::{
    transport::{
        server::{TcpConnectInfo, TlsConnectInfo},
        Certificate, Identity, Server, ServerTlsConfig,
    },
    Request, Status,
};
use x509_parser::{
    certificate::X509Certificate,
    oid_registry::Oid,
    prelude::{oid2abbrev, oid2description, oid2sn, oid_registry},
    traits::FromDer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();

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
    let remote_jobs_server = RemoteJobsServer::with_interceptor(job_service, extract_uid);
    println!("Listening on {}", addr);

    Server::builder()
        .tls_config(tls_config)?
        .add_service(remote_jobs_server)
        .serve(addr)
        .await?;

    Ok(())
}

fn extract_uid(mut req: Request<()>) -> Result<Request<()>, Status> {
    println!("Intercepting request: {:?}", req);

    // extract the client certs
    let client_certs = req
        .peer_certs()
        .ok_or(Status::unauthenticated("Request missing client cert"))?;
    if client_certs.len() == 0 {
        return Err(Status::unauthenticated("Request missing client cert"));
    }

    // get the DER encoded bytes of the cert
    let der = client_certs[0].get_ref(); // rustls encodes the pem as der

    // parse the DER bytes
    let (rem, cert) =
        X509Certificate::from_der(der).map_err(|_| Status::unauthenticated("Bad client cert"))?;
    if rem.is_empty() {
        // parse succeeded
        println!("Got cert subject info: {:?}", cert.subject().to_string());
        let oid: &[u64] = &[0, 9, 2342, 19200300, 100, 1, 1]; // oid for the subject UID.
        let oid = Oid::from(oid).expect("oid parse error: subject uid");
        let uid = cert
            .subject()
            .iter_by_oid(&oid)
            .take(1)
            .next()
            .ok_or(Status::unauthenticated("Client cert missing subject uid"))?;
        if let x509_parser::der_parser::ber::BerObjectContent::UTF8String(user) =
            uid.attr_value().content
        {
            req.extensions_mut().insert(UserExtension {
                user_id: bytes::Bytes::copy_from_slice(user.as_bytes()),
            });
        } else {
            return Err(Status::unauthenticated("Client cert uid must be UTF8"));
        }
    }
    Ok(req)
}

struct UserExtension {
    user_id: bytes::Bytes,
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
