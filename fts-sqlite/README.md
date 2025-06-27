[![crates.io version](https://img.shields.io/crates/v/fts-sqlite.svg)](https://crates.io/crates/fts-sqlite)
[![docs.rs documentation](https://img.shields.io/docsrs/fts-sqlite.svg)](https://docs.rs/fts-sqlite)
[![crates.io downloads](https://img.shields.io/crates/d/fts-sqlite.svg)](https://crates.io/crates/fts-sqlite)
[![crates.io license](https://img.shields.io/crates/l/fts-sqlite.svg)](https://crates.io/crates/fts-sqlite)
[![getting started](https://img.shields.io/badge/🕮_Guide-grey)](https://flowtrading.forwardmarketdesign.com/)

# Flow Trading Service (FTS)

This crate is part of a [collection of crates](https://github.com/forward-market-design/flow-trading-service) that together implement _flow trading_ as proposed
by [Budish, Cramton, et al](https://cramton.umd.edu/papers2020-2024/budish-cramton-kyle-lee-malec-flow-trading.pdf),
in which trade occurs continuously over time via regularly-scheduled batch auctions.

The different crates in this workspace are as follows:

- **[fts_core]**: Defines a set of data primitives and operations but defers the implementations of these operations, consistent with a so-called "hexagonal architecture" approach to separating responsibilities.
- **[fts_solver]**: Provides a reference solver for the flow trading quadratic program.
- **[fts_axum]**: A REST API HTTP server for interacting with the solver and persisting state across auctions.
- **[fts_sqlite]**: An implementation of the core data operations using SQLite, suitable for exploration of flow trading-based marketplaces such as a forward market.

[fts_core]: ../fts-core/README.md
[fts_solver]: ../fts-solver/README.md
[fts_axum]: ../fts-axum/README.md
[fts_sqlite]: ../fts-sqlite/README.md

# FTS SQLite

This crate provides a SQLite-based implementation of all the repository traits defined in `fts-core`, enabling persistent storage and retrieval of flow trading data. It's designed to be efficient for both development/testing scenarios and production use cases with moderate data volumes.

## Architecture

The implementation leverages SQLite's strengths while working around its limitations:

- **Dual connection pools**: Separate reader and writer pools optimize for SQLite's concurrency model
- **WAL mode**: Write-Ahead Logging enables concurrent reads while maintaining consistency
- **Temporal data model**: Built-in support for historical queries and audit trails
- **JSON storage**: Flexible application data storage using SQLite's JSON functions
