# teleport-challenge

## Design 

See the [design document](design.md)

## joblib

This prototype provides a job management library - joblib. See [the joblib README](joblib/README.md)

## server

This prototype includes a server - see [the server README](server/README.md)

## cli

A [cli](cli/README.md) with hard-coded user configuration is provided as well.

## Tests

To run tests: `$ cargo test`

There are tests for joblib and the server api.

The tests are not comprehensive - only basic functionality tests for joblib and authz/authd happy/unhappy path.


## Code Style

This project uses rustfmt with default configuration, which conforms to the [Rust Style Guide](https://github.com/rust-dev-tools/fmt-rfcs/blob/master/guide/guide.md)

## Docs

`$ cargo doc --lib --no-deps --open`
