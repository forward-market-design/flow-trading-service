-- A bidder can update their portfolio's product composition directly, which is
-- managed by the lifetime tables and triggers in the previous migration.
-- However, the effective composition can also change by the creation of new
-- child products. This view synthesizes the changes from the bidder and the
-- product creator, allowing for simplified querying.
create view basis_view (
    portfolio_id, product_id, weight, valid_from, valid_until
) as

select
    portfolio_product.portfolio_id,
    product_tree.dst_id,
    product_tree.ratio * weight as weight,
    max(portfolio_product.valid_from, product_tree.valid_from) as combined_from,
    min(
        coalesce(portfolio_product.valid_until, product_tree.valid_until),
        coalesce(product_tree.valid_until, portfolio_product.valid_until)
    ) as combined_until
from
    portfolio_product
join
    product_tree
    on
        portfolio_product.product_id = product_tree.src_id
where
    combined_until is null or combined_from < combined_until
