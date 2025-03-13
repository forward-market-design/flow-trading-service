# FTS Demo

This crate provides implementations of the data operations defined in `fts-core`. Together with the server implementation in `fts-server`, this crate builds a binary suitable for flow trading functionality. As suggested by the name, correctness and simplicity are prioritized over performance, though the use of SQLite nevertheless enables very fast operations. Products are assumed to correspond to a forward market and are defined by three quantities:

|Property|Description|
|--------|-----------|
|`kind`|A field to distinguish a product variant, such as "FORWARD" or "OPTION"|
|`from`|The time at which the product is to be delivered|
|`thru`|The time at which the delivery will be complete|


It is recommended to pair this binary with a frontend client, such as [this one](https://github.com/forward-market-design/forward-market-demo). The client provides a graphical, administrative interface for familiarizing oneself with the primitives and operations of flow trading and how a forward market might be built upon this foundation.

## Configuration

Run the binary with the `--help` flag to see the available CLI arguments.
```bash
# Build and run the binary in one step
cargo run --release --bin fts-demo -- --help

# OR,
# (1) build the binary...
cargo build --release --bin fts-demo
# ... and (2) run the binary
./target/release/fts-demo --help
```

This output is duplicated below:
```bash
$ fts-demo --help

A simple, reference backend for `fts` implemented with SQLite

Usage: fts-demo [OPTIONS] --api-secret <API_SECRET> --trade-rate <TRADE_RATE>

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
cargo build --release --bin fts-demo --features testmode
```
