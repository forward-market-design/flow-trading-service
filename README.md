# Flow Trading Marketplace

This Cargo workspace contains the various crates that together create a flow
trading marketplace. The individual crates each contain a `README.md` specific
to the crate, but we provide a high-level description below. We also refer the
reader to `CONCEPTS.md`, which explains the flow trading primitives in detail
and how one might build a forward market using these primitives.

## Quick Start

To run an API server using the canonical implementations:
```bash
cargo run --release --bin simple_backend -- --api-secret SECRET --trade-rate 1h
```

This will start a server on port 8080 that uses `SECRET` to decode JWT tokens
and interprets any user input that should be a rate to be with respect to 1 hour.

For more additional configuration options, execute
```bash
cargo run --release --bin simple_backend -- --help
```

A `.env` file can also be utilized to provide the configuration. There is also
a `Dockerfile` that *probably* builds the correct binary.

To see an example of how a client might interact with the server, refer to
`marketplace/tests/roundtrip.rs` for an example that submits bids, schedules a
auction, solves the auction, and queries for the results. In the same
directory, `crud.rs` illustrates some more advanced CRUD operations for a client.

Documentation is provided via an OpenAPI specification. Assuming the above
commands were used to start a server, the specification is available at:

http://localhost:8080/api-docs/openapi.json

and can be viewed in a convenient manner here:

http://localhost:8080/rapidoc

## Project Structure

There are a number of crates in this workspace.

|Crate|Description|
|-----|-----------|
|`marketplace`|The API server implementation, parameterized by a database provider|
|`simple_backend`|A sqlite database provider for `marketplace`|
|`solver`|A generic solver for the (scaled) flow-trading problem|
|`simple_frontend`|(EXPERIMENTAL/WIP) A SvelteKit frontend for the API, for testing and interactive demonstration purposes|
|`pg_backend`|(EXPERIMENTAL) A Postgres database provider for `marketplace`|


The `*_backend` crates create a server binary, knitting together both the `marketplace` and `solver` crates to build a full-featured application. `marketplace` itself is broken down into its `domain` and `server` components; the latter are the endpoints and data handling logic associated to the API server, while the former expressed the core primitives, validation logic, and functionality that a data provider must offer. `solver` is as one would expect; the input to the solver are the *scaled* (by batch duration) submissions for each bidder. That is, `solver` operates on quantities while `marketplace` operates on trade rates. Of course, there is nothing preventing a user from using `solver` as-if it were clearing rates of trade, but the struct definitions and naming reflects the quantity terminology.

Note that building `pg_backend` (which happens when running the `marketplace` tests) requires a local installation of Docker or Podman, as it will automatically launch a Postgres instance to execute queries against. Refer to its `README.md` file for more details.
