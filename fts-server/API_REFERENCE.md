# Forward Market Public API Documentation, v0

This is a high-level overview of the endpoints we are developing. The overall structure and concepts are fairly mature, but the finer points of some of the JSON-outputs might change. "Happy-path" outcomes are documented, assume reasonable errors when this is not the case.

There are a large number of additional endpoints not discussed here:
these "internal" endpoints are for scheduling and executing auction auctions, defining new products, disaggregating existing products, etc.

It can be helpful to think of the API server as combining 3 disparate systems:
1. The "OrderBook" -- a system that manages the state of bids and auction results on the platform
2. The "Ledger" -- a system that manages the settled positions of each user and the products they are trading
3. The "Directory" -- a system that manages product-related metadata, queries, etc.

The Directory module in particular will be completely different based on the application domain (power, bandwidth, etc).

## OrderBook

The orderbook manages a bidder's *submission*. Submissions may be updated at any time, with the most recent version used as the submission for an executing auction. A submission is comprised of two sets of components -- *auths* and *costs*. Without some number of both of these specified, no trade is possible for the bidder.

An auth, short for authorization, is comprised of 3 pieces of information:
1. A globally unique ID.
2. A *portfolio* specification of products and weights, i.e. a direction in product space.
3. *Data* that restricts how this portfolio can be traded.

Once defined, the ID and portfolio are immutable. However, the data attached to the authorization can be updated at any time. This data specifies things like the minimum rate (nonpositive) and maximum rate (nonnegative) the portfolio may be traded at, as well as minimum and maximum trade amounts, so that a bidder may "set and forget" an authorization and have it automatically stop once a certain trade threshold is achieved over potentially many auctions. If an unlimited rates or trades are desired, the bound can be set to `null`, which the system understands to mean the appropriately-signed infinity. 

If no costs reference an auth, we implicitly withhold the authorization from the submission. (If a bidder specifically wishes to trade a portfolio with no cost, they are required to submit a "zero" cost that states so.) To explicitly "stop" an authorization, its data can be set to `null`.

A cost, short for marginal cost curve, is also comprised of 3 pieces of information:
1. A globally unique ID.
2. A *group* specification of authorizations and weights, i.e. a linear combination of authorized portfolios.
3. *Data* that informs the auction how to weigh different potential trades.

As with an auth, once defined the ID and group are immutable while the data attached to the cost can be updated at any time. This data takes 1 of 2 forms -- either a piecewise-linear, weakly monotone decreasing demand curve `{ rate: number, price: number }[]` or a constant curve `{ min_rate: number|null, max_rate: number|null, price: number }`. The domain of these curves must include rate=0; for the constant curve, null bounds correspond to the appropriately-signed infinity.

Costs are used to determine the market clearing prices and optimal trades for each bidder. Essentially, a user's submission defines a function $F(\vec{x}) = \sum_i \int_0^{\vec\omega_i^\top\vec{x}} f_i(y)\,\mathrm{d}y$ where $\vec{x}$ are the trade amounts for each portfolio, and the reported solution satisfies $\mathop{\arg\!\max}_{\vec{x}} f(\vec{x}) - \vec\pi^\top \sum_i \vec\omega_i x_i$ where $\vec\pi$ are the reported per-product prices. (Note that $\vec\pi^\top\vec\omega_i$ defines the portfolio price.) The prices $\vec\pi$ are chosen such that all the trade is balanced with bidders responding optimally as above. Notably each $f_i(y)$ also imposes a domain constraint on the feasible allocation such that $\vec\omega_i^\top\vec{x} \in \mathrm{dom}(f_i)$, in addition to any constraints that the individual auths also impose.

Thus, costs inform the auction how to choose trades. When a cost's group has a single portfolio with weight 1 (which we expect to be the most common expression), the curve acts precisely as a demand curve that states how much to trade of that authorization's portfolio at each possible price. Positive rates correspond to buying and negative rates to selling. Cost groups with two equally-weighted authorizations allow a user to express that the two portfolios are substitutable for one-another, and the auction will choose whichever combination maximizes the overall gains from trade for all bidders.

Submissions will be scaled by the auction's duration to convert a rate-based submission to a quantity-based submission. These two concepts, *auths* and *costs*, allow bidders to express extremely rich demand surfaces across the product space.

## Ledger

(TODO). While auctions produce results, we formally *settle* trades at a slower cadence, perhaps once per day. By settle, we mean that the portfolio trades across a span of auctions are summed, projected to each underlying product, and then rounded to financial precision for each bidder. We also use these rounded adjustments in position to compute the premium payments that must be exchanged between bidders (and facilitated by the market operator) and appropriately debited or credited against an existing collateral balance. If a bidder's collateral balance falls too low, the market operator may impose a policy that stops their submission from participating in further auctions.

The "real-time" results are available through the OrderBook, but the Ledger will maintain the actionable balances. As the OrderBook solves auctions to machine precision while the Ledger rounds to financial precision, there will necessarily be small discrepancies between the implicit OrderBook balances and the explicit Ledger balances. This is both expected and acceptable.

## Directory

(TODO). The directory is managed by the market operator and formally describes the available products to construct authorization portfolios from. From time-to-time, products may be partitioned into subproducts, e.g. a product corresponding to a month of energy delivery partitioning into each of the constituent hours, or a geographical product spanning a region partitioning into metropolitan areas. The directory will maintain the relationships within this tree of products and provide facilities to understand how balances of one might implicitly contribute to balances of its children.

Due to the application-dependence of the directory, there is little more to say, other than that we anticipate products in any forward market to have a dimension corresponding to their delivery interval, but possibly many other dimensions that fully define them.

## Notes

### API Authorization

Most endpoints expect the HTTP header `Authorization: Bearer [JWT TOKEN]`
where [JWT TOKEN] contains a `sub` claim corresponding to the bidder ID the action is being performed on behalf of. We can debate the use of a simpler API token, this is just the current state. I like JWT because they are time-limited, but it's easier to revoke API tokens.

### Number Types

Typically one expects some sort of decimal-type for inputs into trading systems for well established reasons of rounding, precision, etc. However, in our case these inputs go directly into an optimization process that operates in floating point and produces results that are in floating point. Thus, we adopt the philosophy that submission-related quantities are specified in floating point, while ledger-related things are in decimal. That is, things like auths and costs are specified in floating point (as to do otherwise would be somewhat misleading to the users), while the periodic "settlement" process (managed by the Ledger component) aggregates the floating point results across several auction auctions using something like Kahan summation, properly rounds them to a specified precision, and stores them as a decimal-type.

### DateTime and Version types

Authorizations and bids are versioned by the receipt timestamp. All timestamps are normalized to UTC+0 internally, but any RFC3339-compliant datetime string is accepted as input.

### Missing Routes

Presently, there are no Ledger-related routes. This is temporary. They will probably look something like

* `GET|POST /v0/submissions/:BIDDER_ID/collateral`

Get/Set adjustments to the collateral deposited in the system. Obviously the POST method could involve fairly substantial work, e.g. writing to a blockchain. Or maybe this system only implements GET publicly, and instead reads from the blockchain or some other trusted source to gather the collateral information. Notably these numbers will be decimal-typed, not floats.

* `GET /v0/submissions/:BIDDER_ID/positions[?query]`

Returning any matching products and their non-zero positions at this moment. A position is the *settled* position and may lag by a number of auctions. The "unsettled" positions could also be returned as an additional field in the response, so for each product you could get `{ settled_amount: Decimal, pending_amount: float, last_settled: DateTime }` or something.

* `GET /v0/submissions/:BIDDER_ID/positions/:product_id[?query]`

Return more detailed information, such as position-over-time, associated to the specific product.
