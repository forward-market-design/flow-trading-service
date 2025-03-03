
# Flow Trading Concepts

Flow Trading is an approach to trading financial assets invented by
[Budish, Cramton, et al.](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=4145013)
with applications to
[communications](https://www.sciencedirect.com/science/article/pii/S0308596124001174),
[electricity](https://cramton.umd.edu/papers2020-2024/cramton-et-al-forward-energy-market.pdf),
and virtually any commoditized good. In particular, flow trading is a powerful
foundation for a *forward market*, where the products are derivatives of the
day-ahead or real-time products. Rather than trading large quantities in
infrequent, large auctions, flow trading promotes trading small quantities
continuously over large periods of time, which limits market power and establishes
prices and demand for products defined far into the future, enhancing the ability for
participants to plan capacity accordingly.

## Bidders

Any participant (buyer, seller, arbitrageur) in the market is referred to as a bidder.
A bidder has at-most one active [*submission*](#submissions), which is comprised of 1 or more [*auths*](#authorizations-auths)
and 1 or more [*costs*](#marginal-cost-specifications-costs) referencing those auths.

## Products

Different applications will have different definitions of a product, but in a
forward market products will be characterized by a delivery schedule. For example,
in an electricity market a product might be "Forward Energy, Peak, Month of June 2030",
which delivers energy for the peak hours of each day of June 2030. As 2030 approaches,
the market operator may wish to refine this product into the individual hours, e.g.
"Forward Energy, 2030-06-01T10:00-05:00 thru 2030-06-01T11:00-05:00", etc, for each
hour relevant hour. This is encouraged; by design, when defining a new product an optional "parent" and "conversion ratio" may be specified, which allows for portfolios expressed in terms of the parent product to automatically expand to the children. Of course, for non-forward market applications, this capability is strictly optional.


## Authorizations ("auths")

An authorization is one of the two fundamental building blocks of a submission.
At any time, a bidder may define a weighted collection of products ("portfolio")
and authorize trade on this portfolio. This authorization takes the form of
minimum and maximum trade rates (assumed to be +/- infinity if omitted) and
minimum and maximum overall trade totals (again assumed to be +/- infinity if
omitted). A bidder only interested in buying a particular portfolio can specify
a minimum rate of 0, forbidding any sale of it, or similarly for only selling by
setting a maximum rate of 0.

Authorizations may be updated (or removed) at any time as well, if a bidder wants
to adjust their minimum or maximum trade rates or totals or withdraw their
participation. Appropriate constraints are injected into the
[auction clearing mechanism](#auctions) (described below) to ensure outcomes
consistent with the authorizations. There are no restrictions on how many active
authorizations a bidder may have, as well as no restrictions on the contents of
the associated portfolios (products may appear in multiple portfolios, have
positive or negative weights associated to them, etc).

## Marginal Cost Specifications ("costs")

Having defined one or more authorizations, a bidder must then provide the clearing
mechanism information on how to weigh different potential allocations against the
price that would be paid for those allocations. This requires one or more "costs",
which are additive, and defined as follows:

First, each cost must define a *group*. Just like an authorization includes a
portfolio of products, a group is a (weighted) collection of authorizations. The
inner product of these weights with a potential set of trades defines a net rate. For most applications, this group is likely a singleton tuple for one auth with weight 1, but can be as rich as necessary to express substitution or complementarity effects.

Second, each cost must also define a *demand curve*, which specifies the marginal
cost for a net rate. This curve must be weakly monotone decreasing ("non-increasing"),
and contain $r=0$ in its domain. Presently, this curve can be defined in a piecewise-linear fashion in terms of the control points, or as a "constant" curve (min and max rates, and a single constant price). The latter is provided to allow for infinite or half-infinite domains, though we do not anticipate heavy usage of this capability. Indeed, some applications may wish to impose a strict monotonicity requirement with some minimum slope to avoid issues with "flat" demand curves.

Finally, *an auth will not be traded unless it is referenced by at least one cost.* If a particular auth has not "been priced" by a cost, it is unlikely the bidder desires to trade it and this policy avoids a particularly bad outcome. However, if a bidder truly wishes to trade the associated portfolio under the assumption of zero-cost, they can submit a constant cost with the appropriate group, min and max trade rates, and price = 0.

## Submissions

A bidder's collection of active (current) auths and costs define their submission.
We expect a typical submission to consist of a single authorization with a large
portfolio of products, weighted in proportion to their consumption/generation
profile, and a single cost whose group consists of just that one authorization
with weight 1, and a demand curve reflecting their marginal pricing.

Another typical submission may be 1 auth per product (each with a singleton portfolio)
and 1 cost per authorization; there are no restrictions, and bidders may submit any
rich combination of auths and costs that meet their trading needs.

## Auctions

Periodically, the market operator will identify all bidders with active submissions
and run an auction. When an auction is executed, the time of the next auction must
be known -- this is because the submissions, which are defined in terms of rates,
will be scaled by the duration until the next auction to establish submissions in
terms of quantities. Using these scaled submissions, gains from trade are maximized
across all bidders, subject to any constraints established by their auths or costs
and market clearing constraints. This results in clearing prices and individualized
trades across products.

While it is expected that an auction schedule is known to participants, there is no
actual requirement this is the case: all that is necessary is the time of the next
auction. This frees a market operator to make dynamic adjustments to the schedule,
possibly running auctions every few seconds if it makes sense for the market. Bidders
can view results over time by specifying datetime ranges.
