# Prototype Job Worker

## Requirements

### Library

* Worker library with methods to start/stop/query status and get the output of a job.
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

* CLI should be able to connect to worker service and start, stop, get status, and stream output of a job.

## Design

### Library

The library will provide the following functionality:
  * Start Job - spawns a process to run the job on the host.
    * Input: `command`, `args`, `directory`
    * Output `job id` or an error.
  * Stop Job - stops a given job.
    * Input: `job id`
    * Output: void or error if `job id` is not a running job.
  * Query Status - query a job's status.
    * Input: `job id`
    * Output: either `Exited` or `Running`. `Exited` will also contain the exit code of the process. returns an error if `job id` does not exist.
  * Subscribe to Output - registers a subscriber to a job's output stream for `output type` events. Optionally publish all past events to the subscriber. All future events will be published to all subscribers.
    * `output type` can be stdout, stderr, or both.
    * Output: returns an error if `job id` does not exist.
    * Job output is buffered in memory so new subscribers can get the past events as well as future events. A big problem with this is memory exhaustion as job output accumulates. In a real system, I would use a distributed file system to save job output, and a well documented cleanup scheme so users are aware of how long output will persist, or alternatively just set a limit on storage usage for users, and leave it to them to cleanup their files.

### API

* Expose gRPC functions to drive the library.
* gRPC API endpoints will additionally perform authentication and authorization.
* all communication will be secured with mTLS. Security details below.
* See .proto files for message gRPC schema.

### Client CLI

The client CLI will use the gRPC endpoints to interact with the server.
* Predefined users will be created.
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

Usage: client-start --cmd COMMAND --args ARG... --dir DIRECTORY
Usage: client-stop JOBID
Usage: client-status JOBID
Usage: client-output JOBID [ --stdout | --stderr | --all ]

EXAMPLES:
Assume there is a server listening on localhost:1234.
  1. execute "echo hello world". Output is some job id "42".
    $ client -s localhost:1234 -u gavin -c ~/secrets/gavin.pem -k ~/secrets/gavin.key start --cmd "echo hello world" --dir "/tmp"
    42
  2. execute "sleep 10000", which just makes a job that sleeps for 10000 seconds. Outputs job id "77"
    $ client -s localhost:1234 -u gavin -c ~/secrets/gavin.pem -k ~/secrets/gavin.key start --cmd "sleep 10000" --dir "/tmp"
  3. try to stop job 42, but we find it's already completed since "echo hello world" finished basically instantly.
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
### Security

#### Authentication

* X.509 certificates used for mutual authentication, issued by some trusted CA.

#### Authorization

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
  * "Charlie" can start/stop/query jobs of any user.
 
#### Transport Layer

- mTLS using TLS 1.3 with TLS13_AES_256_GCM_SHA384 cipher suite.
  * This uses ECDHE_ECDSA for key exchange/signing. Provides perfect forward secrecy.
  * AES 256 GCM bulk cipher provides strong encryption.
  * Uses a very collision resistant hash algo, SHA-2 w/ 384 bit digest, for message tampering/corruption detection.
  * Faster handshake than TLS 1.2
  * Could use CHACHA20_POLY1305, which might provide some performance benefits for devices with no AES hardware acceleration. I am biased towards AES for its maturity, but both are good.

### Prototype-isms

* User database, management, and roles will not be implemented for simplicity. Instead, just have some pre-defined users hardcoded in the server.
  * A real database for users might just be a simple SQL setup. RBAC can be stored in an SQL database as well. We could also just issue JSON tokens to make permissions stateless but if we allow early invalidation then it doesn't save us anything.
* Certs for both server and clients will be pre-generated and saved in the repo. For a real system, use an actual CA and keep the secrets secret!
* Job output is buffered in memory and never saved to logs.
* All server configuration is hard-coded. If the server crashes for whatever reason, there is no persistence of job info.
  * In a real system, we could keep logs of the running jobs to recover the state of the job coordination server.
* There is only one job worker - the host system itself.
  * To make this thing scale, we could have the server act as a coordinator for many distributed worker systems. There are many approaches and tradeoffs in the implementation of a distributed job worker service that I will not go into. Some inspiration for such a system could be found in the Map Reduce and GFS papers.
