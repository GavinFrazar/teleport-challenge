# joblib

This prototype provides a job management library - joblib.

joblib is designed as an actor system. There are 3 types of actor: `JobCoordinator`, `Worker`, and `Broadcaster`, but only `JobCoordinator` is exposed by the public API.

The `JobCoordinator` is `Send` + `Sync` + `Unpin` + `Clone` and can be freely used from multiple threads in an async context without `Arc<Mutex>`. The reason this is possible is that `JobCoordinator` is actually an actor handle, not the actor itself. It just sends messages across a channel. The actor maintains an in-memory database of jobs by `JobId`. It starts one worker and one broadcaster per job. The worker and broadcaster are likewise just handles to actors.

Each `Worker` manages the life cycle of a job - recording job status (`Running` | `Exited` | `Killed`) and providing a means of killing the job early.
The worker also hooks up the job process stdout/stderr to the sending end of a pipe to a `Broadcaster`.

Each `Broadcaster` manages the output of a job and sending it to all interested parties as a stream of byte blobs. subscribers can specify which stream(s) they are interested in.

The actor model used in this library has a few trade-offs:

### The bad

1. There's a lot of "boilerplate" code to implement the message passing vs just calling functions directly, but the code itself is relatively straightforward.
2. There is more overhead to construction and desconstruction of messages and channels, but it still runs quite fast.
   * I have not tested the real limits of its performance, but it seems to very quickly chug through thousands of messages concurrently in unit tests.

### The good

1. The `JobCoordinator` structure is very convenient to use and requires no locking.
2. There are no locks used (visibly anyway), so a typical deadlock is not possible.
3. Communication deadlock is not possible using this library either.
   * A so-called "communication deadlock" could occur with actors. This is where there is some cycle of bounded `mpsc::channel` in the system with capacity N, but N+1 messages are in-flight - in such a scenario the actors would wait for eachother to receive a message that will never be received.
   * In this implementation, there are no such cycles. `oneshot::channel` and `mpsc::unbounded_channel` do not count towards such cycles, as they never wait to send a message.
   * joblib internally only uses bounded channels to communicate from the `JobCoordinator` handle to its actor, one-way.
5. Resource usage can be controlled easily by the library user.
   * Resource exhaustion is possible when using `mpsc::unbounded_channel` so we need "backpressure" to prevent that.
   * `JobCoordinator` spawns with a configurable limit on the number of messages that can be in its bounded `mpsc::channel` message queue at once. This provides the backpressure we need. It acts like a semaphor, bounding the number of pending requests for the library to process. A user can just `await` calls to `JobCoordinator` async methods with a timeout. If there are too many messages in the queue, the timeout can cancel the request.
      * NOTE: it is still possible for many output streams to overwhelm the system. With more time I would implement a limit on the number of job output streams as well.
6. Race conditions are not possible either, since each actor manages its own resources and processes one message at a time.
7. Extensibility: modelling this library as actors naturally extends to a scaled up distributed model - just change the actor spawn methods to be gRPC calls to remote servers running the actors and pass the same messages over a network. Even the `JobCoordinator` itself could be sharded.
      * NOTE: obviously such a system would not be trivial. All I am saying is that the "actor model" lends itself nicely to such an effort.

### Prototype Limitations

1. JobCoordinator does not exit gracefully - child processes could become zombie processes. This isn't really an actor limitation. Good news though - tokio tries pretty hard to reap child processes, so it's not really an issue.
2. Every job started is persisted in memory, with no cleanup. In a real library, job info would be persisted to a distributed filesystem and resource constraints could be enforced for users.

### Error handling

There a few places in joblib where it just calls `.expect` on a result which is seemingly always going to be the `Ok` variant - usually on the receive end of a channel I know will not have its sender drop/close. For a real library, I would take more care to handle errors, even if they seemingly will never occur, so that the library code will *for sure* not panic unnessarily. joblib does define its own error type and returns that in most places.

### Tests

There are tests in joblib for basic functionality. They can be found in [lib.rs](src/lib.rs)

