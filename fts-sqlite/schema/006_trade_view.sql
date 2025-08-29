-- A bidder's portfolios may "overlap" with respect to the underlying products.
-- Accordingly, we can roll up the net trade for each batch x bidder x product.
create view trade_view (
    batch_id, bidder_id, product_id, trade, price
) as
select
    batch_id, bidder_id, batch_product.product_id, sum(batch_portfolio.trade * portfolio_product.weight * product_tree.ratio), batch_product.price
from
    batch_portfolio
join
    batch_product
using
    (batch_id)
join
    portfolio_product
using
    (portfolio_id)
join
    product_tree
on
    product_tree.src_id = portfolio_product.product_id
and
    product_tree.dst_id = batch_product.product_id
join
    portfolio
on
    portfolio.id = portfolio_id
group by
    batch_id, bidder_id, batch_product.product_id;
--
-- We can also do the same for payments
create view payment_view (
    batch_id, bidder_id, payment
) as
select
    batch.id, bidder_id, sum(batch_portfolio.trade * batch_portfolio.price)
from
    batch_portfolio
join
    portfolio
on
    portfolio.id = batch_portfolio.portfolio_id
group by
    batch_id, bidder_id;