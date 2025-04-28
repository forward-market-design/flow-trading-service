[![crates.io version](https://img.shields.io/crates/v/fts-sqlite.svg)](https://crates.io/crates/fts-sqlite)
[![docs.rs documentation](https://img.shields.io/docsrs/fts-sqlite.svg)](https://docs.rs/fts-sqlite)
[![crates.io downloads](https://img.shields.io/crates/d/fts-sqlite.svg)](https://crates.io/crates/fts-sqlite)
[![crates.io license](https://img.shields.io/crates/l/fts-sqlite.svg)](https://crates.io/crates/fts-sqlite)
[![getting started](https://img.shields.io/badge/🕮_Guide-grey)](https://flowtrading.forwardmarketdesign.com/)

# Flow Trading Service (FTS)

This crate is part of a [collection of crates](https://github.com/forward-market-design/flow-trading-service) that together implement *flow trading* as proposed
by [Budish, Cramton, et al](https://cramton.umd.edu/papers2020-2024/budish-cramton-kyle-lee-malec-flow-trading.pdf),
in which trade occurs continuously over time via regularly-scheduled batch auctions.

The different crates in this workspace are as follows:

- **[fts_core]**: Defines a set of data primitives and operations but defers the implementations of these operations, consistent with a so-called "hexagonal architecture" approach to separating responsibilities.
- **[fts_solver]**: Provides a reference solver for the flow trading quadratic program.
- **[fts_server]**: A REST API HTTP server for interacting with the solver and persisting state across auctions.
- **[fts_sqlite]**: An implementation of the core data operations using SQLite, suitable for exploration of flow trading-based marketplaces such as a forward market.

[fts_core]: ../fts-core/README.md
[fts_solver]: ../fts-solver/README.md
[fts_server]: ../fts-server/README.md
[fts_sqlite]: ../fts-sqlite/README.md


# FTS Demo

This crate provides implementations of the data operations defined in `fts-core`. Products are assumed to correspond to a forward market and are defined by three quantities:

|Property|Description|
|--------|-----------|
|`kind`|A field to distinguish a product variant, such as "FORWARD" or "OPTION"|
|`from`|The time at which the product is to be delivered|
|`thru`|The time at which the delivery will be complete|

