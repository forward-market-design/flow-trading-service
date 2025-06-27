create table product (
    id text primary key,
    -- the datetime at which this product was created
    as_of text not null,
    -- arbitrary data for a client application to associate
    app_data blob not null, -- Json<Any>
    -- products can optionally have parents
    parent_id text,
    parent_ratio real default 0.0 not null,
    foreign key (parent_id) references product (id)
) strict;
--
-- products can be decomposed over time
-- this table represents a transitive closure over the hierarchical tree of products
create table product_tree (
    -- ancestor product
    src_id text not null,
    -- descendent product
    dst_id text not null,
    -- value(ancestor) * ratio = value(descendent)
    ratio real not null,
    -- location(ancestor) - location(descendant) = depth >= 0
    depth integer not null,
    -- when was this relation created?
    valid_from text not null,
    valid_until text,
    -- indices necessary for efficient querying
    primary key (src_id, dst_id, valid_from),
    unique (dst_id, src_id, valid_from),
    unique (valid_from, valid_until, src_id, dst_id),
    -- make sure we're referencing real products
    foreign key (src_id) references product (id),
    foreign key (dst_id) references product (id)
) strict, without rowid;
--
-- This trigger fully maintains the product_tree table as we insert new
-- products
create trigger product_tree_trigger
after insert on product
begin
-- Invalidate existing paths to the parent
update product_tree
set
    valid_until = new.as_of
where
    dst_id = new.parent_id;

-- Extend paths ending in the parent id to include the new product linters sometimes have troubles
-- with the "new" keyword in triggers. Here, we exclude it from the linting rules, and below we don't have to.
-- noqa: disable=RF01
insert into product_tree (
    src_id,
    dst_id,
    ratio,
    depth,
    valid_from,
    valid_until
)
select
    pt.src_id,
    new.id,
    pt.ratio * new.parent_ratio as ratio,
    pt.depth + 1 as depth,
    new.as_of,
    null as valid_until
from
    product_tree as pt
where
    pt.dst_id = new.parent_id;
-- noqa: enable=RF01

-- Insert the "leaf" row for the product
insert into product_tree (
    src_id,
    dst_id,
    ratio,
    depth,
    valid_from,
    valid_until
)
values (
    new.id,
    new.id,
    1.0,
    0,
    new.as_of,
    null
);
end;
