# Prototype Job Service

## Requirements

### Library

* Job service library with methods to start/stop/query status and get the output of a job.
* Library should be able to stream the output of a running job.
  * Output should be from start of process execution.
  * Multiple concurrent clients should be supported.

### API

* [GRPC](https://grpc.io) API to start/stop/get status/stream output of a running process.
* Use mTLS authentication and verify client certificate. Set up strong set of
  cipher suites for TLS and good crypto setup for certificates. Do not use any
  other authentication protocols on top of mTLS.
* Use a simple authorization scheme.

### Client

* CLI should be able to connect to job service and start, stop, get status, and stream output of a job.

## Library

The library will provide a JobCoordinator structure. The JobCoordinator will keep records of jobs by `job id`: status, output, command, args, dir, envs.
JobCoordinator will hand out `job id`, from a simple integer counter, as jobs are started. It will also provide the following thread-safe functionality:
  * Start Job - spawns a process to run the job on the host.
    * Input: `command`, `args`, `directory`, `envs`
    * Output `job id`
      * NOTE: `job id` will be a simple uint64 number starting from 0 and incrementing as jobs are started. A real implementation would use a real UUID. This would especially matter if we scaled this to a distributed job service. For now, the counter will just be an int behind a mutex.
    * Error When: Job fails to start for any reason: command not found, directory not found, etc.
      * NOTE: if a command exits with an error, maybe because of bad usage like `touch` - that does not return error from this function. The command still spawns, it just exits with an error code immediately.
  * Stop Job - stops a given job.
    * Input: `job id`
    * Output: void
    * Error When: `job id` does not exist or is not a *running* job.
  * Query Status - query a job's status.
    * Input: `job id`
    * Output: either `Exited` or `Running`. `Exited` will also contain the exit code of the process.
    * Error When: `job id` does not exist.
  * Subscribe to Output - registers a subscriber to a job's output stream for `output type` events. Publish all past events to the subscriber. All future events will be published to all subscribers.
    * `output type` can be stdout, stderr, or both.
    * Input: `job id`, 
    * Output: some Outputstream object/channel. TBD
    * Error When: `job id` does not exist.
    * Job output is buffered in memory so new subscribers can get the past events as well as future events. A big problem with this is memory exhaustion as job output accumulates. In a real system, I would use a distributed file system to save job output, and a well documented cleanup scheme so users are aware of how long output will persist, or alternatively just set a limit on storage usage for users and leave it to them to cleanup their files.

## API

* Expose gRPC functions as a server listening on 127.0.0.1:[SomePort] to drive the library.
* All communication between client/server will be secured with mTLS. See [Security details](authentication) below.
* Server authenticates users using their cert.
* Server maintains `job id` -> `entity` and `user` -> `entity` -> `roles` records for authorization inside gRPC calls.
* See [jobservice.proto](protobuf/jobservice.proto) for message and service schema.

## Client CLI

The client CLI will use the gRPC endpoints to interact with the server.
* Predefined users will be created.
* For now, just make user/cert/key cli options. TODO: add config files so the cli isn't tedious to use
* cert/key authenticate the user, server then checks user's roles for authorization.
* Usage should look roughly like:

```sh
Usage: client [OPTIONS] [SUBCOMMAND]

OPTIONS:
  -s, --server <url>
  -u, --user <username>
  -c, --cert <certificate>
  -k, --key <key>

SUBCOMMANDS:
  start
  stop
  status
  output
  
SUBCOMMAND Details:

Usage: client-start --cmd COMMAND --args ARG... --dir DIRECTORY --envs ENV=VAL...
Usage: client-stop JOBID
Usage: client-status JOBID
Usage: client-output JOBID [ --stdout | --stderr | --all ]

EXAMPLES:
Assume there is a server listening on localhost:1234.
  1. execute "echo hello world". Output is job id "42".
    $ client -s localhost:1234 -u gavin -c ~/secrets/gavin.pem -k ~/secrets/gavin.key start --cmd "echo" --args "hello world" --envs PATH="/usr/bin" --dir "/tmp"
    42
  2. execute "sleep 10000", which just makes a job that sleeps for 10000 seconds. Outputs job id "77"
    $ client -s localhost:1234 -u gavin -c ~/secrets/gavin.pem -k ~/secrets/gavin.key start --cmd "sleep" --args "10000" --envs PATH="/usr/bin" --dir "/tmp"
  3. try to stop job 42, but we find it is already completed since "echo hello world" finished basically instantly.
    $ client -s localhost:1234 -u gavin -c ~/secrets/gavin.pem -k ~/secrets/gavin.key stop 42
    Job '42' is not running.
  4. Get job status.
    $ client -s localhost:1234 -u gavin -c ~/secrets/gavin.pem -k ~/secrets/gavin.key status 42
    Job '42': Exited: 0
    $ client -s localhost:1234 -u gavin -c ~/secrets/gavin.pem -k ~/secrets/gavin.key status 77
    Job '77': Running
  5. stop the sleep job
    $ client -s localhost:1234 -u gavin -c ~/secrets/gavin.pem -k ~/secrets/gavin.key stop 77
    Stopped Job '77'
  6. Get output
    $ client -s localhost:1234 -u gavin -c ~/secrets/gavin.pem -k ~/secrets/gavin.key output 42 --all
    hello world
    $ client -s localhost:1234 -u gavin -c ~/secrets/gavin.pem -k ~/secrets/gavin.key output 77 --all
  7. $ client -s localhost:1234 -u gavin -c ~/secrets/gavin.pem -k ~/secrets/gavin.key status 77
    Job '77': Exited: 130
```

### Nice to have

* allow users to list jobs on the system. Gated by RBAC as well. I may implement this but I'm being mindful of scope creep.

## Authentication

* X.509 certificates used for mutual authentication, signed by some trusted CA.
* For the project I will pre-generate keys and sign them for the server/client/CA as if I were a root CA (the CA cert will be signed by the CA itself).

## Authorization

- Simplified Role Based Access Control
  * Permissions:
    * Start/Stop Job
    * Query Job Status/Output
  * Roles:
    * Task Manager: start/stop and status/output permissions.
    * Analyst: status/output permissions only.
  * Entities:
    * Self - the entity corresponding to an invididual user. The only users with permissions here are the individual user and those with "All" roles.
    * All - entity corresponding to *all* entities.
  * Users are assigned roles per entity.
  * Could add distinct shared entities, but I want to keep it simple. Also, it does not seem useful without real resource isolation/control, which is out of scope. For now there is only one "shared entity": the host system itself.
- Example setup

| User | Entity | Role |
| :---: | :---: | :---:|
| Alice | Self | Task Manager |
| Bob | All | Analyst |
| Charlie | All | Task Manager |

  * "Alice" can start/stop/query jobs of her own.
  * "Bob" cannot start/stop jobs. He can, however, query jobs of other users.
  * "Charlie" can start/stop/query jobs of any user. NOTE: if Charlie starts a job, it will be for his Self entity. There is no start-job impersonation.
 
## Transport Layer

- mTLS using TLS 1.3 with TLS13_AES_256_GCM_SHA384 cipher suite.
  * This uses ECDHE_ECDSA for key exchange/signing. Provides perfect forward secrecy.
  * AES 256 GCM bulk cipher provides strong encryption.
  * Uses a very collision resistant hash algo, SHA-2 w/ 384 bit digest, for message tampering/corruption detection.
  * Faster handshake than TLS 1.2
  * Could use CHACHA20_POLY1305, which might provide some performance benefits for devices with no AES hardware acceleration. I am biased towards AES for its maturity, but both are good.

## Prototype-isms

* User database, management, and roles will not be implemented for simplicity. Instead, just have some pre-defined users hardcoded in the server.
  * A real database for users might just be a simple SQL setup. RBAC can be stored in an SQL database as well. We could also just issue JSON tokens to make permissions stateless but if we allow early invalidation then it doesn't save us anything.
* Certs for both server and clients will be pre-generated and saved in the repo. For a real system, use an actual CA and keep the secrets secret!
* Job output is buffered in memory and never saved to logs.
* All server configuration is hard-coded. If the server crashes for whatever reason, there is no persistence of job info.
  * In a real system, we could keep logs of the running jobs to recover the state of the job coordination server.
* There is only one job worker - the host system itself.
  * To make this thing scale, we could have the server act as a coordinator for many distributed worker systems.
