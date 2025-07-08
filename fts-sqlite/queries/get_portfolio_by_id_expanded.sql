-- fn(demand_id: DemandId, as_of: DateTime) -> DemandRow
with
app_data_cte as (
    select
        id as portfolio_id,
        bidder_id,
        app_data as value,
        as_of
    from
        portfolio
    where
        id = $1
),

demand_group_cte as (
    select
        portfolio_id,
        max(valid_from) as valid_from,
        min(valid_until) as valid_until,
        jsonb_group_object(demand_id, weight) as value
    from
        demand_group
    where
        portfolio_id = $1
        and
        valid_from <= $2
        and
        ($2 < valid_until or valid_until is null)
    group by
        portfolio_id
),

product_group_cte as (
    select
        portfolio_id,
        max(valid_from) as valid_from,
        min(valid_until) as valid_until,
        jsonb_group_object(product_id, weight) as value
    from
        product_group_view
    where
        portfolio_id = $1
        and
        valid_from <= $2
        and
        ($2 < valid_until or valid_until is null)
    group by
        portfolio_id
)

select
    portfolio_id as "id!: PortfolioId",
    max(
        coalesce(demand_group_cte.valid_from, product_group_cte.valid_from, app_data_cte.as_of),
        coalesce(product_group_cte.valid_from, demand_group_cte.valid_from, app_data_cte.as_of)
    ) as "valid_from!: DateTime",
    min(
        coalesce(demand_group_cte.valid_until, product_group_cte.valid_until),
        coalesce(product_group_cte.valid_until, demand_group_cte.valid_until)
    ) as "valid_until?: DateTime",
    app_data_cte.bidder_id as "bidder_id!: BidderId",
    json(app_data_cte.value) as "app_data!: sqlx::types::Json<PortfolioData>",
    json(demand_group_cte.value) as "demand_group?: sqlx::types::Json<DemandGroup<DemandId>>",
    json(product_group_cte.value) as "product_group?: sqlx::types::Json<ProductGroup<ProductId>>"
from
    app_data_cte
left join
    demand_group_cte
    using
        (portfolio_id)
left join
    product_group_cte
    using
        (portfolio_id);
