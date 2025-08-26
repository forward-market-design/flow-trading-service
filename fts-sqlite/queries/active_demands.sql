-- A demand is considered active if and only if
-- * it has non-null curve data, AND
-- * it is associated to at least 1 portfolio.
with
portfolio_by_id as (
    select
        demand_id,
        valid_until as expires
    from
        portfolio_demand
    where
        valid_from <= $1
    and
        ($1 < valid_until or valid_until is null)
),

curve_data_by_id as (
    select
        demand_id,
        valid_until as expires,
        value
    from
        curve_data
    where
        value is not null
    and
        valid_from <= $1
    and
        ($1 < valid_until or valid_until is null)
)

select
    demand_id as "id!: DemandId",
    min(
        coalesce(portfolio_by_id.expires, curve_data_by_id.expires),
        coalesce(curve_data_by_id.expires, portfolio_by_id.expires)
    ) as "expires?: DateTime",
    json(curve_data_by_id.value) as "value!: sqlx::types::Json<DemandCurveDto>"
from
    portfolio_by_id
join
    curve_data_by_id
using
    (demand_id)