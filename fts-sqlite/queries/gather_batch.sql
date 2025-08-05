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
),

pg as (
        select
            portfolio_id,
            jsonb_group_object(product_id, weight) as basis
        from
            basis_view
        where
            valid_from <= $1 and ($1 < valid_until or valid_until is null)
        group by
            portfolio_id
        having
            count(*) > 0
),

portfolios as (
    select
        jsonb_group_object(pg.portfolio_id, jsonb_array(coalesce(demand_group, jsonb_object()), basis)) as portfolios
    from
        pg
    left join
        dg
    on
        dg.portfolio_id = pg.portfolio_id 
)

select
    json(demand_curves) as "demands?: sqlx::types::Json<Map<DemandId, DemandCurveDto>>",
    json(portfolios) as "portfolios?: sqlx::types::Json<Map<PortfolioId, (DemandGroup<DemandId>, Basis<ProductId>)>>"
from
    dc
full join
    portfolios
