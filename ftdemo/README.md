# `ftdemo`

This crate combines the SQLite implementation of `fts-sqlite` with the REST API  of `fts-server` to product a binary suitable for interactive demonstration of flow trading functionality. As suggested by the name, correctness and simplicity are prioritized over performance, though the use of SQLite nevertheless enables very fast operations.

It is recommended to pair this binary with a frontend client, such as [this one](https://github.com/forward-market-design/flow-trading-client). The client provides a graphical, administrative interface for familiarizing oneself with the primitives and operations of flow trading and how a forward market might be built upon this foundation.

## Configuration

Run the binary with the `--help` flag to see the available CLI arguments.
```bash
# Build and run the binary in one step
cargo run --release --bin ftdemo -- --help

# OR,
# (1) build the binary...
cargo build --release --bin ftdemo
# ... and (2) run the binary
./target/release/ftdemo --help
```

This output is duplicated below:
```bash
$ ftdemo --help

A demonstrative implementation of a flow trading server

Usage: ftdemo [OPTIONS] --api-secret <API_SECRET> --trade-rate <TRADE_RATE>

Options:
      --api-port <API_PORT>      The port to listen on [env: API_PORT=] [default: 8080]
      --api-secret <API_SECRET>  The HMAC-secret for verification of JWT claims [env: API_SECRET=]
      --database <DATABASE>      The location of the database (if omitted, use an in-memory db) [env: DATABASE=]
      --trade-rate <TRADE_RATE>  The time unit of rate data [env: TRADE_RATE=]
  -h, --help                     Print help
  -V, --version                  Print version
```

As suggested by this output, a `.env` file may alternatively be provided to specify these configuration options (useful for container-based deployments). 

Note that `--trade-rate / TRADE_RATE` is specified as a string that can be parsed by [humantime](https://docs.rs/humantime/latest/humantime/), e.g. `1h` or `30min`. This value provides the units for *auths* and *costs*, e.g. if an auth specifies a `max_rate` of `5` and the server was configured with `--trade-rate 1h`, then this means the authorization allows for trading the associated portfolio at a rate not exceeding 5 units per hour.

For convenience, a compile-time feature (disabled by default) is available, that when enabled, adds a `--test N` flag which will print JWT tokens to stdout for 1 admin user and `N` randomly generated bidders, valid for 1 day, for use in external tooling and testing scenarios. Use with the appropriate care. To enable support, build with the `testmode` feature:

```bash
cargo build --release --bin ftdemo --features testmode
```
