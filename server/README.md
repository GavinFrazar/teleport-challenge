## Remote Job Server

This binary provides a gRPC remote jobs server. It leverages the joblib library to manage jobs.

## Authentication

Authentication is done with mTLS using TLS 1.3 and secured by the TLS13_AES_256_GCM_SHA384 cipher suite.

Originally I designed this prototype to have one root CA, but during implementation I realized that best practice is to at least use one
CA for server certs and one CA for client certs.

I still kept the chain simple though - just a root CA for each, but in a real implementation we would use intermediate certs.

## Authorization

I used a mock database of user->scope->roles, role->permissions, and jobid->owner, pre-populated with a few users.

## Protobuf

Protobuf codegen is done using tonic-build and prost.

I made some name changes to the protobuf file/package when, after codegen, it became apparent that some names were poorly chosen.

## tests

I included tests in [main.rs](src/main.rs)

Tests cover basic functionality and authd/authz happy/unhappy paths. 
