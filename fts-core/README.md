[![crates.io version](https://img.shields.io/crates/v/fts-core.svg)](https://crates.io/crates/fts-core)
[![docs.rs documentation](https://img.shields.io/docsrs/fts-core.svg)](https://docs.rs/fts-core)
[![crates.io downloads](https://img.shields.io/crates/d/fts-core.svg)](https://crates.io/crates/fts-core)
[![crates.io license](https://img.shields.io/crates/l/fts-core.svg)](https://crates.io/crates/fts-core)
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
[ftdemo]: ../ftdemo/README.md

# FTS Core

This crate defines a set of data primitives and operations but defers the implementations of these operations, consistent with a so-called "hexagonal architecture" approach to separating responsibilities.

Broadly speaking, this core defines 4 objects as well as actions related to these objects. These are:
1. Demand curves: weakly monotone decreasing functions that specify a marginal cost as a function of trade rate.
1. Portfolios, which are
    1. a vector in product space, defining a direction to trade in, and
    1. a weighted collection of demand curves which this portfolio's trade contributes to.

Furthermore, periodically the demand curves and portfolios are *batched* and submitted to an auction for execution. The results of this auction include the optimal rates of trade for each portfolio and the clearing prices for each product corresponding to that batch.

## Demand Curves

A _demand curve_ represents a bidder's interest in trading by expressing a price as a function of a net rate. This function must be (1) weakly monotone decreasing, and (2) include `rate=0` in its domain.

The demand curve can be specified in two ways:

- **Piecewise-linear (PWL)**: A series of (rate, price) points defining a weakly monotone decreasing curve,
- **Constant**: A fixed price over a rate interval, useful for expressing indifference to trade rates at a specific price.

The sign convention follows flow trading standards:

- Positive rates represent buying
- Negative rates represent selling

For example, a demand curve might express:

```text
Rate: -100  -50   0   50   100
Price:  10   9.5  9.0   8.5    7.5
```

This indicates the bidder is willing to sell at higher prices and buy at lower prices, with 9.0 as their neutral price.

Demands can be updated by changing their curve data. Setting the curve data to `null` effectively deactivates the demand while preserving its history.

## Portfolios

A _portfolio_ acts as a container that groups together related demands and products for trading. It serves two key functions:

1. **Demand aggregation**: Groups multiple demands with weights, allowing complex trading strategies
2. **Product association**: Specifies which products can be traded and their relative weights

### Product Weights

Suppose a market contains products `A` and `B`. A portfolio's basis might be:

```typescript
// Trade only product A
basis1 = { A: 1.0 };
// Trade A and B in strictly equal amounts
basis2 = { A: 1.0, B: 1.0 };
// Replace A with B (or vice versa) at some fixed ratio
basis3 = { A: 0.5, B: -0.75 };
```
There are no restrictions on the signs or magnitudes of the weights. Buying 1 unit of portfolio `basis3` corresponds to buying 0.5 units of `A` and selling 0.75 units of `B`.

### Demand Weights

Similarly, a portfolio can aggregate multiple demands:

```typescript
// We expect most often that a portfolio is
// associated to a single demand, and vice versa
demand_group1 = { D1: 1.0 };
// However, if portfolios are substitutes, multiple
// portfolios may be associated to a single demand,
// and a single portfolio may be associated to multiple
// demands.
demand_group2 = { D1: 1, D2: 1 };
// As before, there are no restrictions
// on the signs or magnitudes of these weights.
demand_group3 = { D1: 0.8, D2: 0.2 };
```

This allows bidders to express substitution preferences between different pricing strategies while maintaining a unified trading interface.

Once created, a portfolio's associations are updated through the `update_portfolio` method, which can modify either the demand group or product group independently.

## Auctions

Neither `fts-core` nor any of the companion crates in this workspace impose any restrictions on the frequency of batched auctions, though it may desirable to post a public schedule execute the auctions on a recurring basis.

The output of an auction includes:

- The quantities traded for each portfolio
- The clearing prices for each product

The trade of individual products can be computed from these outputs, as well as other statistics of interest.

The optimization requires that the net trade of each product is exactly zero. These are returned as raw 64 bit floats and are not suitable for financial applications as-is: likely some sort of "settlement" process should be built on top of this, which aggregates trades across multiple auctions and rounds them in an appropriate manner. This settlement scheme is left for the market operator to implement.

## Products

Flow trading by itself imposes no restrictions on the products actually being traded. They are simply "things" that are referenced by portfolio product groups and constrained to net-zero trade in each auction. With that said, this project was built with forward markets in mind. The sibling crate `ftdemo` imposes a product structure characterized by a product `kind`, and an interval in time over which the product is delivered. Details are in [ftdemo], but the takeaway is that the requirements of a forward market led to an additional concept in the core implementation.

This concept is that of a "product hierarchy", where a market operator can decompose products over time. For example, a product corresponding to energy delivery for the month of June 2030 might, at a later time, be refined into products for each day of June 2030, and at an even later time, each day into its hours. A portfolio defined only in terms of the monthly product needs an ability to implicitly decompose into the child products when an auction is executed in order to properly clear the market. This is useful beyond forward markets: consider trading stocks pre- and post-split. `fts-core` imposes a requirement that portfolios are always retrieved with respect to the "contemporary product basis."
