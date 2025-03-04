# FTS Demo

The sibling crate `fts-core` defines a core set of data primitives and
operations, but defers the actual implementation of these operations. The
sibling crate `fts-server` builds a full-featured REST service on top of a
concrete implementation of these operations. This crate, `fts-demo`, is such an
implementation, knitting it together with `fts-server` to provide a server binary.

As suggested by the name, this implementation is intended to demonstrate the
capabilities of flow trading. Accordingly, correctness and simplicity are
prioritized over performance, though the use of SQLite allows for very fast operations.

It is recommended to pair this binary with a frontend client, such as [this one](https://github.com/forward-market-design/fts-client). The client provides a graphical interface for familiarizing oneself with the primitives and operations of flow trading.

## Configuration

Run the binary with the `--help` flag to see the available CLI arguments. This
output is duplicated below:
```
$ fts-demo --help

A simple, reference backend for `fts` implemented with SQLite

Usage: fts-demo [OPTIONS] --api-secret <API_SECRET> --trade-rate <TRADE_RATE>

Options:
      --api-port <API_PORT>      The port to listen on [env: API_PORT=] [default: 8080]
      --api-secret <API_SECRET>  The HMAC-secret for verification of JWT claims [env: API_SECRET=]
      --database <DATABASE>      The location of the orderbook database (if omitted, use an in-memory db) [env: DATABASE=]
      --trade-rate <TRADE_RATE>  The time unit of rate data [env: TRADE_RATE=]
  -h, --help                     Print help
  -V, --version                  Print version
```

As indicated by the help output, these arguments can alternatively be specified via a `.env` file, useful for container-based deployments. 

Note that `--trade-rate` is specified as a string that can be parsed by [humantime](https://docs.rs/humantime/latest/humantime/), e.g. `1h` or `30min`. This value provides the units for *auths* and *costs*, e.g. if an auth specifies a `max_rate` of `5` and the server was configured with `--trade-rate 1h`, then this means the authorization allows for trading the associated portfolio at a rate not exceeding 5 units per hour.

For convenience, a compile-time feature (disabled by default) is available, that when enabled, adds a `--test N` flag which will print credentials for 1 admin user and `N` randomly generated bidders, valid for 1 day, for use in external tooling and testing scenarios. Use with the appropriate care. To enable support, build with the `testmode` feature:
```bash
cargo build --release --bin fts-demo --features testmode
```
