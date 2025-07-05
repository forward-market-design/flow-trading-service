-- A demand is considered active if and only if
-- * it has non-null curve data, AND
-- * it is associated to at least 1 portfolio.
with
portfolio_groups_by_id as (
    select
        demand_id,
        max(valid_from) as valid_from,
        min(valid_until) as valid_until,
        jsonb_group_object(portfolio_id, weight) as pgroup
    from
        demand_group
    where
        valid_from <= $1
    and
        ($1 < valid_until or valid_until is null)
    group by
        demand_id
),

curve_data_by_id as (
    select
        demand_id,
        valid_from,
        valid_until,
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
    max(curve_data_by_id.valid_from, portfolio_groups_by_id.valid_from) as "valid_from!: DateTime",
    min(
        coalesce(portfolio_groups_by_id.valid_until, curve_data_by_id.valid_until),
        coalesce(curve_data_by_id.valid_until, portfolio_groups_by_id.valid_until)
    ) as "valid_until?: DateTime",
    bidder_id as "bidder_id!: BidderId",
    json("null") as "app_data!: sqlx::types::Json<()>",
    json(curve_data_by_id.value) as "curve_data?: sqlx::types::Json<DemandCurveDto>",
    json(pgroup) as "portfolio_group?: sqlx::types::Json<PortfolioGroup<PortfolioId>>"
from
    portfolio_groups_by_id
join
    curve_data_by_id
using
    (demand_id)
join
    demand
on
    demand.id = demand_id
