with dc as (
    select
        jsonb_group_object(demand_id, value) as demand_curves
    from
        curve_data
    where
        valid_from <= $1 and ($1 < valid_until or valid_until is null) and value is not null
),

dg as (
    select
        jsonb_group_object(portfolio_id, demand_group) as demand_groups
    from (
        select
            portfolio_id,
            jsonb_group_object(demand_id, weight) as demand_group
        from
            demand_group
        where
            valid_from <= $1 and ($1 < valid_until or valid_until is null)
        group by
            portfolio_id
        having
            count(*) > 0
    )
),

pg as (
    select
        jsonb_group_object(portfolio_id, product_group) as product_groups
    from (
        select
            portfolio_id,
            jsonb_group_object(product_id, weight) as product_group
        from
            product_group_view
        where
            valid_from <= $1 and ($1 < valid_until or valid_until is null)
        group by
            portfolio_id
        having
            count(*) > 0
    )
)

select
    json(demand_curves) as "demand_curves?: sqlx::types::Json<Map<DemandId, DemandCurveDto>>",
    json(demand_groups) as "demand_groups?: sqlx::types::Json<Map<PortfolioId, Map<DemandId>>>",
    json(product_groups) as "product_groups?: sqlx::types::Json<Map<PortfolioId, Map<ProductId>>>"
from
    dc
full join dg full join pg
