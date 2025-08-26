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

demand_cte as (
    select
        portfolio_id,
        max(valid_from) as valid_from,
        min(valid_until) as valid_until,
        jsonb_group_object(demand_id, weight) as value
    from
        portfolio_demand
    where
        portfolio_id = $1
        and
        valid_from <= $2
        and
        ($2 < valid_until or valid_until is null)
    group by
        portfolio_id
),

basis_cte as (
    select
        portfolio_id,
        max(valid_from) as valid_from,
        min(valid_until) as valid_until,
        jsonb_group_object(product_id, weight) as value
    from
        portfolio_product
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
        coalesce(demand_cte.valid_from, basis_cte.valid_from, app_data_cte.as_of),
        coalesce(basis_cte.valid_from, demand_cte.valid_from, app_data_cte.as_of)
    ) as "valid_from!: DateTime",
    min(
        coalesce(demand_cte.valid_until, basis_cte.valid_until),
        coalesce(basis_cte.valid_until, demand_cte.valid_until)
    ) as "valid_until?: DateTime",
    app_data_cte.bidder_id as "bidder_id!: BidderId",
    json(app_data_cte.value) as "app_data!: sqlx::types::Json<PortfolioData>",
    json(demand_cte.value) as "demand?: sqlx::types::Json<Weights<DemandId>>",
    json(basis_cte.value) as "basis?: sqlx::types::Json<Basis<ProductId>>"
from
    app_data_cte
left join
    demand_cte
    using
        (portfolio_id)
left join
    basis_cte
    using
        (portfolio_id);
