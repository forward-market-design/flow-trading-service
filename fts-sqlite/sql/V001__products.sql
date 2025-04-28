--
-- A very basic product type
create table product (
    id blob primary key,
    "from" text not null,
    thru text not null,
    kind text not null,
    timestamp text not null,
    unique (kind, "from", thru)
);
--
-- While the orderbook doesn't have a semantic notion of products, it does
-- maintain a (historical) record of the products as they are increasingly
-- refined. The reason for doing so is to allow for costs to persist as-is
-- across such a refinement: portfolios may refer to products that have been
-- refined. Accordingly, we need the ability to implicitly expand these
-- onto the appropriate product basis as needed.
--
-- This is managed by maintaining the transitive closure of the product
-- hierarchy. Here, `src` is an ancestor, `dst` is a descendant, `depth` is the
-- tree-distance between the two, and `ratio` maps 1 unit of the src to however
-- many units of the dst.
--
-- For our simple products, we can entirely manage this table via triggers.
--
create table product_tree (
    -- ancestor product
    src blob not null,
    -- descendent product
    dst blob not null,
    -- value(ancestor) * ratio = value(descendent)
    ratio real not null,
    -- location(ancestor) - location(descendant) = depth >= 0
    depth integer not null,
    -- when was this relation created?
    timestamp text not null,
    -- indices necessary for efficient querying
    primary key (src, dst),
    unique (dst, src),
    -- make sure we're referencing real products
    foreign key (src) references product (id),
    foreign key (dst) references product (id)
) strict,
without rowid;
--
create trigger product_tree_trigger
after
insert on product begin
-- Find all rows whose `dst` would be considered a parent
-- to the new product, then extend the paths to the new child.
-- If we're honest with ourselves, it should be the case that
-- at most 1 product could be considered a parent!
insert into
    product_tree (src, dst, ratio, depth, timestamp)
select
    product_tree.src,
    new.id,
    product_tree.ratio * 1.0,
    product_tree.depth + 1,
    new.timestamp
from
    product_tree
join
    product
on
    product_tree.dst = product.id
where
    product.kind = new.kind
and
    product."from" <= new."from"
and
    product.thru >= new.thru;
-- Also insert the "root" row for the child itself
insert into product_tree (src, dst, ratio, depth, timestamp) values (new.id, new.id, 1.0, 0, new.timestamp);
--
end;