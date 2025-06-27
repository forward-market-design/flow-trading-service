[![crates.io version](https://img.shields.io/crates/v/fts-axum.svg)](https://crates.io/crates/fts-axum)
[![docs.rs documentation](https://img.shields.io/docsrs/fts-axum.svg)](https://docs.rs/fts-axum)
[![crates.io downloads](https://img.shields.io/crates/d/fts-axum.svg)](https://crates.io/crates/fts-axum)
[![crates.io license](https://img.shields.io/crates/l/fts-axum.svg)](https://crates.io/crates/fts-axum)
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

# FTS Axum Server

This crate provides a REST API for the core flow trading operations. A running server will host this schema at http://localhost:8080/docs.

## On the use of JSON and HTTP

It is true that JSON is a significantly flawed choice for (de)serialization of
bid data. It is also true that a RESTful API over HTTP is questionable, at
best, with respect to building a trading platform. On the other hand, these
choices allow for virtually any programming environment to easily interface
with the server, as well as open the door to rich, web-based clients.

Given that this project is primarily intended to _motivate_ the use of flow trading, especially in the context of forward markets, these trade-offs are more than reasonable. With that said, the design of flow trading specifically discourages high-frequency execution, so the performance overhead of these trade-offs are also largely irrelevant.

## Authorization

In the interest of simplicity, endpoints that process bid data (or execute administrative actions) expect HTTP requests to contain the `Authorization` header with a bearer token. While an implementation is free to choose the token format, a good choice is a JWT token where the `sub:` claim specifies the bidder's UUID, alongside any additional claims.

## API Endpoints and Data Types

Please refer to the automatically generated OpenAPI schema for up-to-date documentation of the endpoints. Note that any endpoint expecting a datetime type expects an RFC3339-compliant string.
