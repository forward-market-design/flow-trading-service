# FTS Core

This crate defines a set of data primitives and operations but defers the implementations of these operations, consistent with a so-called "hexagonal architecture" approach to separating responsibilities.

Broadly speaking, this core defines the notion of a *bidder*, whom may have at most one [*submission*](#submissions). This submission contains any number of [*auths*](#auths) and [*costs*](#costs). Periodically, every active submission is submitted into an [*auction*](#auctions); the submissions are collectively processed into an optimization program which is then solved. The results of this optimization are reported as the traded amounts of each auth and the prices for the underlying [*products*](#products).

## Auths

An *auth*, short for authorization, is two things:

1. A definition of a *portfolio*, which is a weighted bundle of products.
2. Constraints on how this portfolio can be traded.

Suppose a market contains products `A` and `B` (among others). The following are all examples of portfolios:
```typescript
// A "singleton" portfolio that trades only A
P1 = { A: 1.0 }
// A portfolio that trades both A and B **in equal amounts**
P2 = { A: 1.0, B: 1.0 }
// A portfolio that replaces A with B (or vice versa)
P3 = { A: 0.5, B: -0.75 }
```

There are no restrictions on the signs or magnitudes of the "weights". Buying 1 unit of a portfolio will buy 1 times the associated weight for each underlying product; selling 1 unit will do the same. (Flow trading utilizes the convention of selling as negative trade and buying as positive trade, so buying 4 units of `P3` corresponds to buying 2 units of `A` and selling 3 units of `B`.)

Unbounded trade (even when bound proportionally as above) is rarely desired. Thus, an auth also allows for specifying minimum and maximum rates of trade and total trade. These minima and maxima are specified with respect to the trade sign convention; coupling the portfolio `P1` to `min_rate = 0` restricts the auth to only the buying of `P1` (and in turn `A`): selling is forbidden. Conversely, setting `max_rate = 0` would restrict the auth to only the selling of `P1`. It is important to note that this is only a statement of what *this* auth is allowed to trade; if `A` is *also* involved in another auth's portfolio, the underlying product may still be bought or sold if that auth allows for it. The only restriction on these constraints is that `min_rate <= 0 <= max_rate`. (If omitted, they default to the appropriately signed infinity.)

We also support minimum and maximum overall trade: as each submission is being prepared for an auction, an implementation will consider the overall total trade that has occurred across all previous auctions and additionally impose any further restriction on minimum and maximum rates such that the minimum and maximum trade amounts are respected. That is, at each auction the following calculation is performed for each auth:
```text
min_trade <- min(0, max(min_trade - current_trade, min_rate * duration))
max_trade <- max(0, min(max_trade - current_trade, max_rate * duration))
```

Note that each auction has an associated duration, e.g. if an auction is scheduled for every hour, the duration is 1 hour.

Once defined, an auth's portfolio is immutable. To adjust the portfolio, the auth must be stopped and a new auth created. However, the constraints ("auth data") can be updated at any time. The auth's most current data is included whenever a submission is compiled. To stop an auth, its data can simply be set to `null`. (If the desire is to temporarily suspend an auth, one might instead set `min_rate = max_rate = 0`).

## Costs

The [flow trading paper](https://cramton.umd.edu/papers2020-2024/budish-cramton-kyle-lee-malec-flow-trading.pdf) does not explicitly consider what we have termed auths and costs; instead, a piecewise-linear demand curve is directly associated to a portfolio. We have added one layer of separation (into auths and costs) to support the ability for a bidder to express substitution preferences between portfolios. In our implementation, a cost is two things:

1. A definition of a cost *group*, which is a linear combination of auths.
2. A piecewise-linear demand *curve*.

The latter is exactly as it is in the paper, with the positive/negative trade convention. Unlike the paper, there is no restriction on this curve being strictly monotone decreasing (though a marketplace implementation can certainly choose to reject "flat" demand curves). If flat curves are instead a desired feature, the curve may alternatively be specified as a `(min_rate, max_rate, price)` triplet. Notably the sibling crate `fts-solver` does not presently provide any explicit tie-breaking procedures (though that is intended, future functionality), so operators are discouraged from allowing flat curves if `fts-solver` is being used as the auction solver.

The former requires some explanation. Suppose we have defined portfolios `P = { A: 1 }` and `Q = { B: 1 }`. Conflating the portfolio name with the auth id, suppose our group is defined as `G = { P: 1, Q: 1 }`. This expresses that `A` and `B` are perfect substitutes for one another. If the optimization decides that we should trade 2 units of `G`, then this could be achieved by trading 1 unit each of `P` and `Q`, or 2 units of `P` and 0 of `Q`, or 3 of `P` and -1 of `Q`, and so-forth -- subject to the auth constraints and market clearing, of course.

As with portfolios, there are no restrictions on the magnitude or sign of the group weights, and multiple groups can refer to the same auths. Also as with portfolios, a cost group is immutable once defined: to change a group, the cost must be stopped (by setting its data to `null`) and a new cost created. A cost's data can be updated at any time.

## Submissions

A submission is merely the collection of a bidder's auths and costs, with one additional consideration: If an auth is not explicitly referenced by any cost, it is removed from the submission. Without an explicit cost, the auth would otherwise trade freely, which is typically undesirable. If it is, in fact, desired the bidder can simply construct a cost with a flat demand curve at `price = 0`.

## Auctions

An integral component to flow trading is that of a regularly scheduled, batch auction. It is possible to clear submissions in terms of rates and execute a new auction on every submission update, but there are good reasons to prefer a fixed, recurring schedule. Namely, this prevents high-frequency traders from gaining an edge over "normal" participants and removes the incentives for otherwise-expensive low-latency networking and optimization.

Neither `fts-core` nor any of the companion crates in this workspace impose any restrictions on the frequency of auctions. They are executed "on-demand" by an external actor. We recommend this actor post a schedule and execute auctions accordingly, but they are free to dynamically change the batch duration, execute on every change, or whatever they wish. This works because auths and costs are defined with respect to *rates*, so adjusting to a new batch duration is as simple as scaling bids by a different number. All that is required is that the time of the *next* auction is known, so that any minimum/maximum trade restrictions from the auths can be enforced correctly.

The output of an auction are the quantities traded of each auth's portfolio (*not* the underlying products, though those can be easily constructed) and the prices of each product. The optimization requires that the net trade of each product is exactly zero. These are returned as raw 64 bit floats and are not suitable for financial applications as-is: likely some sort of "settlement" process should be built on top of this, which aggregates trades across multiple auctions and rounds them in an appropriate manner. This settlement scheme is left for the market operator to implement.

## Products

Flow trading by itself imposes no restrictions on the products actually being traded. They are simply "things" that are referenced by auth portfolios and constrained to net-zero trade in each auction. With that said, this project was built with forward markets in mind. The sibling crate `fts-demo` necessarily imposes a product structure, which happens to correspond to a forward market. Details are in [fts_demo], but the takeaway is that the requirements of a forward market led to an additional concept in the core implementation.

This concept is that of a "product hierarchy", where an operator can decompose products over time. For example, a product corresponding to energy delivery for the month of June 2030 might, at a later time, be refined into products for each day of June 2030, and at an even later time, each day into its hours. A portfolio defined only in terms of the monthly product needs an ability to implicitly decompose into the child products when an auction is executed in order to properly clear the market. This is useful beyond forward markets: consider trading stocks pre- and post-split. Thus, when retrieving portfolios, `fts-core` provides implementers an affordance to control which portfolio is returned (the explicit portfolio, the implicit portfolio, or none at all).

[fts_demo]: ../fts-demo/README.md