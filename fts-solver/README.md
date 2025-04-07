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
- **[fts_server]**: A REST API HTTP server for interacting with the solver and persisting state across auctions.
- **[fts_demo]**: An implementation of the core data operations using SQLite, suitable for exploration of flow trading-based marketplaces such as a forward market.

[fts_core]: ../fts-core/README.md
[fts_solver]: ../fts-solver/README.md
[fts_server]: ../fts-server/README.md
[fts_demo]: ../fts-demo/README.md

# FTS Solver

This package defines a few basic types and a solver interface to operate over these types. Presently, the following solvers are provided:
* `feature = ["clarabel"]` -- Uses the [Clarabel](https://clarabel.org/) interior point solver for the quadratic program
* `feature = ["osqp"]` -- Uses the [OSQP](https://osqp.org/) ADMM solver for the quadratic program

Additional solvers will be developed as needed. The present implementations are intended as "reference" for future work.

## Primitive Types

There are two externally-defined types `ProductId` and `AuthId`, which allow the application host to provide their own implementations. These are black-boxes as far as the solver is concerned -- they just need to implement `Clone + Eq + Hash + Ord`.

This crate defines a `Submission<AuthId, ProductId>` type, which is intended to encapsulate a single bidder's submission. A submission is a combination of *auths* and *costs*: an auth defines a portfolio (a sparse vector over product space) and the minimum and maximum allowable trade of this portfolio. Costs define a linear combination of auths (a group) and a utility function, whose domain additionally constrains the space of feasible outcomes. It is assumed by the solver that auth ids are globally unique; that is, the auth ids should be disjointly partitioned amongst the submissions. (It does not otherwise cause an error, but will likely yield unexpected results.) Refer to the implementations of `src/types/auth.rs` and `src/types/cost.rs` for more details on these types. Refer to `tests/simple_solve.rs` for an example assembling two submissions and solving them.


## TODO

* Warm-start interface
* Large-scale tests
* Enhanced dual reporting
* Automatic determination of error tolerances based on input
* Bespoke ADMM implementation
