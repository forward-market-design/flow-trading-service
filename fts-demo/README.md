# Simple Backend

The sibling package `marketplace` defines a `MarketRepository` trait and
a generic HTTP server that is parameterized by an implementation thereof.

This package is that implementation: all of the IO operations required to
implement a flow trading service. In particular, this package prioritizes the
following considerations in order:
1. Correctness.
2. Simplicity.
3. Performance.

Correctness, because `marketplace` depends on this package for testing, then
simplicity because we are still exploring the implementation space and
simulating markets, and performance last, although this implementation should
easily handle real-time markets with thousands of bidders.

For information on the API server, refer to the `marketplace` documentation:
requests are authenticated via JWT with the `sub:` claim set to the
`bidder_id`, and admin routes are restricted to tokens with `admin: true` in
the claims.

Run the binary with the `--help` flag to see the available CLI arguments. This
output is duplicated below:
```
$ simple_backend --help

A simple, reference backend for `marketplace` implemented with SQLite

Usage: simple_backend [OPTIONS] --api-secret <API_SECRET> --trade-rate <TRADE_RATE>

Options:
      --api-port <API_PORT>      The port to listen on [env: API_PORT=] [default: 8080]
      --api-secret <API_SECRET>  The HMAC-secret for verification of JWT claims [env: API_SECRET=]
      --database <DATABASE>      The location of the orderbook database (if omitted, use an in-memory db) [env: DATABASE=]
      --trade-rate <TRADE_RATE>  The duration of time rates are specified with respect to [env: TRADE_RATE=]
  -h, --help                     Print help
  -V, --version                  Print version
```

Note that `--trade-rate` is specified in as a string that can be parsed by [humantime](https://docs.rs/humantime/latest/humantime/), e.g. `1h` or `30min`.

For convenience, a compile-time feature (disabled by default) is available, that when enabled also adds a `--test N` flag which will print credentials for 1 admin user and `N` randomly generated bidders, valid for 1 day, for use in external tooling and testing scenarios. Use with the appropriate care. To enable support, build with the `testmode` feature:
```bash
cargo build --release --bin simple_backend --features testmode
```

## TODO

There are lots of blog posts about "the right flags" to use with SQLite. The current flags are a first attempt at incorporating some of this best practice, but these can likely be improved. Relevant resources:
  * https://lobste.rs/s/fxkk7v/why_does_sqlite_production_have_such_bad
  * https://kerkour.com/sqlite-for-servers
  * https://gcollazo.com/optimal-sqlite-settings-for-django/
  * https://lobste.rs/s/rvsgqy/gotchas_with_sqlite_production
  * https://blog.pecar.me/sqlite-prod