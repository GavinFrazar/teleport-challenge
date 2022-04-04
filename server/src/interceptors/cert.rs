use crate::services::jobservice::UserId;
use tonic::{Request, Status};
use x509_parser::{certificate::X509Certificate, oid_registry::Oid, traits::FromDer};

/// A tonic interceptor service function.
///
/// Extracts the subject uid from the client certificate and adds it to the request extensions.
pub fn extract_subj_uid(mut req: Request<()>) -> Result<Request<()>, Status> {
    // extract the client certs
    let client_certs = req
        .peer_certs()
        .ok_or_else(|| Status::unauthenticated("Request missing client cert"))?;
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
        let oid: &[u64] = &[0, 9, 2342, 19200300, 100, 1, 1]; // oid for the subject UID.
        let oid = Oid::from(oid).expect("oid parse error: subject uid");
        let uid = cert
            .subject()
            .iter_by_oid(&oid)
            .take(1)
            .next()
            .ok_or_else(|| Status::unauthenticated("Client cert missing subject uid"))?;
        if let x509_parser::der_parser::ber::BerObjectContent::UTF8String(user) =
            uid.attr_value().content
        {
            req.extensions_mut().insert(UserExtension {
                user_id: String::from(user),
            });
        } else {
            return Err(Status::unauthenticated("Client cert uid must be UTF8"));
        }
    }
    Ok(req)
}

pub struct UserExtension {
    pub user_id: UserId,
}
