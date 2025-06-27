[![crates.io version](https://img.shields.io/crates/v/fts-solver.svg)](https://crates.io/crates/fts-solver)
[![docs.rs documentation](https://img.shields.io/docsrs/fts-solver.svg)](https://docs.rs/fts-solver)
[![crates.io downloads](https://img.shields.io/crates/d/fts-solver.svg)](https://crates.io/crates/fts-solver)
[![crates.io license](https://img.shields.io/crates/l/fts-solver.svg)](https://crates.io/crates/fts-solver)
[![getting started](https://img.shields.io/badge/🕮_Guide-grey)](https://flowtrading.forwardmarketdesign.com/)


# Flow Trading Service (FTS)

This crate is part of a [collection of crates](https://github.com/forward-market-design/flow-trading-service) that together implement *flow trading* as proposed
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

# FTS Solver

This package defines a few basic types and a solver interface to operate over these types. Presently, the following solvers are provided:
* `feature = ["clarabel"]` -- Uses the [Clarabel](https://clarabel.org/) interior point solver for the quadratic program
* `feature = ["osqp"]` -- Uses the [OSQP](https://osqp.org/) ADMM solver for the quadratic program

Additional solvers will be developed as needed. The present implementations are intended as "reference" for future work.

There are a few additional features exposed by this crate. If an application intends to (de)serialize the primitive data types directly,
enabling `feature = ["serde"]` will provide Serde bindings.

## Primitive Types

There are three externally-defined types `DemandId`, `PortfolioId`, and `ProductId`, which allow the application host to provide their own implementations. These are black-boxes as far as the solver is concerned -- they just need to implement `Clone + Eq + Hash + Ord`.

Note that the solver has no notion of a bidder: all portfolios and demand are treated together. A user of this library is responsible for reassociating the outcomes to the individual bidders.

A portfolio is characterized by two quantities:
1. A vector in product space (typically sparse) that defines a trading direction -- trade of products can only occur along portfolio directions.
2. A vector in demand space (typically sparse). Each portfolio is associated to one or more demand curves; each demand curve sums the associated, weighted portfolio trades in determining the marginal cost.

## TODO

* Warm-start interface
* Large-scale tests
* Enhanced dual reporting
* Automatic determination of error tolerances based on input
* Bespoke ADMM implementation
