## Remote Job Server

This binary provides a gRPC remote jobs server. It leverages the joblib library to manage jobs.

## Authentication

Authentication is done with mTLS using TLS 1.3 and secured by the TLS13_AES_256_GCM_SHA384 cipher suite.

Originally I designed this prototype to have one root certificate, but during implementation I realized that best practice is to at least use one
for server/client cert signing each.

## Authorization

I used a mock database of user->scope->roles, role->permissions, and jobid->owner, pre-populated with a few users.

## Protobuf

Protobuf codegen is done using tonic-build and prost.

I made some name changes to the protobuf file/package when, after codegen, it became apparent that some names were poorly chosen.

## tests

I included tests in [main.rs](src/main.rs)

Tests can be run from the workspace, which will
