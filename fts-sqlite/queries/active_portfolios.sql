-- A portfolio is considered active if and only if
-- * it has at least one associated demand, OR
-- * it has at least one associated product.
with
demand_groups_by_id as (
    select
        portfolio_id,
        max(valid_from) as valid_from,
        min(valid_until) as valid_until,
        jsonb_group_object(demand_id, weight) as dgroup
    from
        demand_group
    where
        demand_group.valid_from <= $2
    and
        ($2 < demand_group.valid_until or demand_group.valid_until is null)
    group by
        portfolio_id
),

product_groups_by_id as (
    select
        portfolio_id,
        max(valid_from) as valid_from,
        min(valid_until) as valid_until,
        jsonb_group_object(product_id, weight) as pgroup
    from
        product_group
    where
        product_group.valid_from <= $2
    and
        ($2 < product_group.valid_until or product_group.valid_until is null)
    group by
        portfolio_id
)

select
    portfolio_id as "id!: PortfolioId",
    max(
        coalesce(demand_groups_by_id.valid_from, product_groups_by_id.valid_from),
        coalesce(product_groups_by_id.valid_from, demand_groups_by_id.valid_from)
     ) as "valid_from!: DateTime",
    min(
        coalesce(demand_groups_by_id.valid_until, product_groups_by_id.valid_until),
        coalesce(product_groups_by_id.valid_until, demand_groups_by_id.valid_until)
    ) as "valid_until?: DateTime",
    bidder_id as "bidder_id!: BidderId",
    json(app_data) as "app_data!: sqlx::types::Json<PortfolioData>",
    json(dgroup) as "demand_group?: sqlx::types::Json<DemandGroup<DemandId>>",
    json(pgroup) as "product_group?: sqlx::types::Json<ProductGroup<ProductId>>"
from
    demand_groups_by_id
full join
    product_groups_by_id
using
    (portfolio_id)
join
    portfolio
on
    portfolio_id = portfolio.id
join
    json_each($1) as bidder_ids
on
    portfolio.bidder_id = bidder_ids.atom
