-- fn(demand_id: DemandId, as_of: DateTime) -> DemandRow
with
app_data_cte as (
    select
        id as demand_id,
        bidder_id,
        app_data as value,
        as_of
    from
        demand
    where
        id = $1
),

curve_data_cte as (
    select
        demand_id,
        valid_from,
        valid_until,
        value
    from
        curve_data
    where
        demand_id = $1
        and
        valid_from <= $2
        and
        ($2 < valid_until or valid_until is null)
),

portfolio_group_cte as (
    select
        demand_id,
        max(valid_from) as valid_from,
        min(valid_until) as valid_until,
        jsonb_group_object(portfolio_id, weight) as value
    from
        demand_group
    where
        demand_id = $1
        and
        valid_from <= $2
        and
        ($2 < valid_until or valid_until is null)
    group by
        demand_id
)

select
    demand_id as "id!: DemandId",
    max(
        coalesce(curve_data_cte.valid_from, portfolio_group_cte.valid_from, app_data_cte.as_of),
        coalesce(portfolio_group_cte.valid_from, curve_data_cte.valid_from, app_data_cte.as_of)
     ) as "valid_from!: DateTime",
    min(
        coalesce(curve_data_cte.valid_until, portfolio_group_cte.valid_until),
        coalesce(portfolio_group_cte.valid_until, curve_data_cte.valid_until)
    ) as "valid_until?: DateTime",
    app_data_cte.bidder_id as "bidder_id!: BidderId",
    json(app_data_cte.value) as "app_data!: sqlx::types::Json<DemandData>",
    json(curve_data_cte.value) as "curve_data?: sqlx::types::Json<DemandCurveDto>",
    json(portfolio_group_cte.value) as "portfolio_group?: sqlx::types::Json<PortfolioGroup<PortfolioId>>"
from
    app_data_cte
left join
    curve_data_cte
    using
        (demand_id)
left join
    portfolio_group_cte
    using
        (demand_id);
