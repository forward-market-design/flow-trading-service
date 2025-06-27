-- fn(demand_id: DemandId, as_of: DateTime) -> DemandRow
with
app_data_cte as (
    select
        id as demand_id,
        bidder_id,
        app_data as value
    from
        demand
    where
        id = $1
),

curve_data_cte as (
    select
        demand_id,
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
    app_data_cte.bidder_id as "bidder_id!: BidderId",
    json(app_data_cte.value) as "app_data!: sqlx::types::Json<DemandData>",
    json(curve_data_cte.value) as "curve_data?: sqlx::types::Json<DemandCurveDto>",
    json(portfolio_group_cte.value) as "portfolio_group?: sqlx::types::Json<Map<PortfolioId>>"
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
