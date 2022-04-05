# Remote Job Service CLI

## Usage
```
cli 
Connect to a gRPC job server

USAGE:
    cli --user <USER> --server <SERVER> <SUBCOMMAND>

OPTIONS:
    -h, --help               Print help information
    -s, --server <SERVER>    The address of the server
    -u, --user <USER>        user name (selects user cert/key from a hard-coded path TODO: real
                             implementation use real config file)

SUBCOMMANDS:
    help      Print this message or the help of the given subcommand(s)
    output    stream a job's output
    start     start a new job
    status    get a job's status
    stop      stop a job
```

```
cli-start 
start a new job

USAGE:
    cli start [OPTIONS] --command <COMMAND> --dir <DIR> [--] [ARGS]...

ARGS:
    <ARGS>...    

OPTIONS:
    -c, --command <COMMAND>    name of the command to run
    -d, --dir <DIR>            working directory for the command
    -e, --envs <ENVS>...       list of environment variables
    -h, --help                 Print help information
```

```
cli-stop 
stop a job

USAGE:
    cli stop <JOB_ID>

ARGS:
    <JOB_ID>    Uuid v4 string

OPTIONS:
    -h, --help    Print help information
```

```
cli-status 
get a job's status

USAGE:
    cli status <JOB_ID>

ARGS:
    <JOB_ID>    Uuid v4 string

OPTIONS:
    -h, --help    Print help information
```

```
cli-output 
stream a job's output

USAGE:
    cli output <OUTPUT_TYPE> <JOB_ID>

ARGS:
    <OUTPUT_TYPE>    type of output to stream [possible values: stdout, stderr, all]
    <JOB_ID>         Uuid v4 string

OPTIONS:
    -h, --help    Print help information
```

## Examples

* NOTE: the cli didnt exactly match the design. I realized I needed a way to handle job option args,
        so to make parsing easier we have args as positionally at the end of the start subcommand.
        If flags are needed, you must pass "-- -arg1" for example.
```
# Assume there is a server listening on [::1]:50051

## start a job to execute "echo -n hello world -- hi", which prints "hello world -- hi" with no trailing newline.

$ uuid=$(./cli -u alice -s "[::1]:50051" start --command echo --dir "/tmp" -- -n hello world -- hi)
$ ./cli -u alice -s [::1]:50051 stop $uuid
Error: Status { code: Internal, message: "Job already stopped", metadata: MetadataMap { headers: {"content-type": "application/grpc", "date": "Tue, 05 Apr 2022 08:43:05 GMT"} }, source: None }
$ ./cli -u alice -s [::1]:50051 status $uuid
Exited with code: 0
$ ./cli -u alice -s [::1]:50051 output all $uuid
hello world$
```
