-- A portfolio is considered active if and only if
-- * it has at least one associated demand, AND
-- * it has at least one associated product.
with
demand_by_id as (
    select
        portfolio_id,
        max(valid_from) as valid_from,
        min(valid_until) as valid_until,
        jsonb_group_object(demand_id, weight) as dgroup
    from
        portfolio_demand
    where
        valid_from <= $1
    and
        ($1 < valid_until or valid_until is null)
    group by
        portfolio_id
),

basis_by_id as (
    select
        portfolio_id,
        max(valid_from) as valid_from,
        min(valid_until) as valid_until,
        jsonb_group_object(product_id, weight) as pgroup
    from
        basis_view
    where
        valid_from <= $1
    and
        ($1 < valid_until or valid_until is null)
    group by
        portfolio_id
)

select
    portfolio_id as "id!: PortfolioId",
    max(demand_by_id.valid_from, basis_by_id.valid_from) as "valid_from!: DateTime",
    min(
        coalesce(demand_by_id.valid_until, basis_by_id.valid_until),
        coalesce(basis_by_id.valid_until, demand_by_id.valid_until)
    ) as "valid_until?: DateTime",
    bidder_id as "bidder_id!: BidderId",
    json("null") as "app_data!: sqlx::types::Json<()>",
    json(dgroup) as "demand?: sqlx::types::Json<Weights<DemandId>>",
    json(pgroup) as "basis?: sqlx::types::Json<Basis<ProductId>>"
from
    demand_by_id
join
    basis_by_id
using
    (portfolio_id)
join
    portfolio
on
    portfolio_id = portfolio.id
