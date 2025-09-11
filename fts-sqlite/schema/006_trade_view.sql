-- A bidder's portfolios may "overlap" with respect to the underlying products.
-- Accordingly, we can roll up the net trade for each batch x bidder x product.
create view trade_view (
    batch_id, settlement_id, bidder_id, product_id, trade_from, trade_until, trade, price
) as
select
    batch.id,
    batch.settlement_id,
    portfolio.bidder_id,
    batch_product.product_id,
    batch.valid_from,
    batch.valid_until,
    sum(batch_portfolio.trade * portfolio_product.weight * product_tree.ratio) / batch.time_unit_in_ms,
    batch_product.price
from
    batch
join
    batch_portfolio
on
    batch.id = batch_portfolio.batch_id
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
    batch_id, settlement_id, bidder_id, trade_from, trade_until, payment
) as
select
    batch.id,
    batch.settlement_id,
    portfolio.bidder_id,
    batch.valid_from,
    batch.valid_until,
    sum(batch_portfolio.trade * batch_portfolio.price) / batch.time_unit_in_ms
from
    batch
join
    batch_portfolio
on
    batch.id = batch_portfolio.batch_id
join
    portfolio
on
    portfolio.id = batch_portfolio.portfolio_id
group by
    batch_id, bidder_id;