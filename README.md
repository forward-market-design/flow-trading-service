# Flow Trading Service (FTS)

This project represents a complete software implementation of *flow trading* as proposed
by [Budish, Cramton, et al](https://cramton.umd.edu/papers2020-2024/budish-cramton-kyle-lee-malec-flow-trading.pdf),
in which trade occurs continuously over time via regularly-scheduled batch auctions.

Users of the software are encouraged to visit our [documentation website](https://flowtrading.forwardmarketdesign.com/).
The rest of this README, and the other README files in child directories, are
geared towards introducing the software architecture and design decisions of this implementation instead of foundational flow trading concepts.

Much of this technical documentation is also available here: https://docs.rs/fts-core/latest/fts_core

## Overview

We define a core set of primitives in `fts-core` (so-called
"models" and "ports", using the terminology of hexagonal architecture), a
reference solver for the relevant quadratic program in `fts-solver`, a REST API HTTP server for interacting with the solver in `fts-server`, and
finally an implementation of the core data operations in `fts-sqlite` using
SQLite, suitable for exploration of flow trading-based marketplaces such as a forward market.

These 4 crates each contain their own `README.md` which explains the crate's functionality and the relevant high-level design. We explicitly call out `fts-core/README.md` as an introduction to the bid primitives used in our flow trading implementation.
