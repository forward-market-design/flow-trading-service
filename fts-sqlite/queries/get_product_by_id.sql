-- fn(product_id: ProductId, as_of: DateTime) -> ProductRow
select
    product.id as "id!: ProductId",
    json(product.app_data) as "app_data!: sqlx::types::Json<ProductData>",
    case
        when
            product.parent_id is null
        then
            null
        else
            json_array(product.parent_id, product.parent_ratio)
        end as "parent?: sqlx::types::Json<(ProductId, f64)>",
    json_group_object(product_tree.dst_id, product_tree.ratio) as "basis!: sqlx::types::Json<Basis<ProductId>>"
from
    product
join
    product_tree
on
    product.id = product_tree.src_id
where
    product.id = $1
and
    product_tree.valid_from <= $2
and
    ($2 < product_tree.valid_until or product_tree.valid_until is null)
group by
    product.id