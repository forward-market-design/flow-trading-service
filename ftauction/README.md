# `ftauction`

This crate is a light wrapper around `fts-solver`, providing a binary that can
read flow trading-based auctions from a standardized JSON input format and writes
their solution to stdout or a file. This binary also makes available other useful operations,
such as exporting the intermediate quadratic program to a standardized output format
for analysis in other tools.

The JSON format is very simple. Given the following type definitions:
```typescript
type ProductId = string;
type PortfolioId = string;

type Sum = Record<DemandId, number> | Array<DemandId> | DemandId;
type Basis = Record<ProductId, number> | Array<ProductId> | ProductId;
// The canonical types in Sum and Basis are Records, but we implicitly
// transform arrays and values for convenience according to:
//   X => { X: 1.0 },
//   [X, Y, ...] => { X: 1.0, Y: 1.0, ...}

type Portfolio = { demand: Sum, basis: Basis };

type Point = {
    rate: number,
    price: number,
}

type DemandCurve = Array<Point> | { min_rate?: null | number, max_rate?: null | number, price: number };

type Auction = {
    demand_curves: Record<DemandId, DemandCurve>,
    portfolios: Record<PortfolioId, Portfolio>,
}
```

The JSON input format is simply anything that deserializes as
`Auction`. Examples can be found in the [`fts-solver` test suite](https://github.com/forward-market-design/flow-trading-service/tree/main/fts-solver/tests/samples).

## Installation

To install, simply run `cargo install ftauction`.
To build from source, `cargo build --release --bin ftauction`.

## Usage

All options are documented in `ftauction --help` and `ftauction [SUBCOMMAND] --help`.

Some examples:

```bash

# Solve an auction given a file
ftauction solve -o solution.json input.json

# Solve an auction over stdin
curl http://some.remote/file.json | ftauction solve -o solution.json -

# Read an auction over stdin, export to stdout
cat input.json | ftauction export - --format mps
```

The ordering between the input (a path, or "-") and the flags ("--format", for example) is not important.
