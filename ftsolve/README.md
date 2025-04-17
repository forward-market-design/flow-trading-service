# `ftsolve`

This crate is a light wrapper around `fts-solver`, providing a binary that can
read flow trading-based auctions from a standardized JSON input format and writes
their solution to stdout or a file. This binary also makes available other useful operations,
such as exporting the intermediate quadratic program to a standardized output format
for analysis in other tools.

The JSON format is very simple. Given the following type definitions:
```typescript
type ProductId = string;
type PortfolioId = string;

type Portfolio = Record<ProductId, number> | Array<ProductId> | ProductId;
type Group = Record<PortfolioId, number> | Array<PortfolioId> | PortfolioId;
// The canonical types in Portfolio and Group are Records, but we implicitly
// transform arrays and values for convenience according to:
//   X => { X: 1.0 },
//   [X, Y, ...] => { X: 1.0, Y: 1.0, ...}

interface Point {
    quantity: number,
    price: number,
}

interface DemandCurve {
    // If omitted, `domain` is computed from `points`.
    // If provided, the points will be interpolated or extrapolated accordingly.
    // Use `null` as a stand-in for ±∞
    domain?: [number | null, number | null];

    group: Group,
    points: Array<Point>,
}

interface Submission {
    portfolios: Record<PortfolioId, Portfolio>,
    demand_curves: Array<DemandCurve>,
}
```

The JSON input format is simply anything that deserializes as
`Record<BidderId, Submission>`. Examples can be found in the [`fts-solver` test suite](https://github.com/forward-market-design/flow-trading-service/tree/main/fts-solver/tests/samples).

## Installation

To install, simply run `cargo install ftsolve`.
To build from source, `cargo build --release --bin ftsolve`.

Execute `ftsolve --help` to see options.