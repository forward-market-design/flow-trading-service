# Flow Trading Service (FTS)

This project represents a complete implementation of "flow trading" as proposed
by [Budish, Cramton, et al](https://cramton.umd.edu/papers2020-2024/budish-cramton-kyle-lee-malec-flow-trading.pdf).
In particular, it defines a core set of primitives in `fts-core` (so-called
"models" and "ports", using the terminology of hexagonal architecture), a
reference solver for the associated quadratic program in `fts-solver`, a basic,
RESTful HTTP server for interacting with the solver in `fts-server`, and
finally an implementation of the core data operations in `fts-demo` using
SQLite, suitable for exploration of flow-trading based marketplaces.

These 4 crates each contain their own `README.md` and explain
the functionality of the associated crate and the relevant high-level concepts.

## Quick Start

To get started, ensure Rust >= 1.85 is available in your system `PATH`. (See [Rustup](https://rustup.rs/) for an easy way to install Rust.) Then, paste the following into your CLI:
```bash
cargo run --release --bin fts-demo -- --api-secret SECRET --trade-rate 1h
```

This will download the project dependencies, build the software, and then run the server on port 8080. The OpenAPI specification will be available at http://localhost:8080/rapidoc if the server is successfully running. A Dockerfile is also available to build and run the binary.

Refer to `fts-demo/README.md` for specific documentation what arguments are available and what they mean.

