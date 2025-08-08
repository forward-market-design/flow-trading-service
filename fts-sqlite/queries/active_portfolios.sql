-- A portfolio is considered active if and only if
-- * it has at least one associated demand, AND
-- * it has at least one associated product.
with
demand_by_id as (
    select
        portfolio_id,
        valid_until as expires,
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
        valid_until as expires,
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
    min(
        coalesce(demand_by_id.expires, basis_by_id.expires),
        coalesce(basis_by_id.expires, demand_by_id.expires)
    ) as "expires?: DateTime",
    json(dgroup) as "demand!: sqlx::types::Json<Weights<DemandId>>",
    json(pgroup) as "basis!: sqlx::types::Json<Basis<ProductId>>"
from
    demand_by_id
join
    basis_by_id
using
    (portfolio_id)
