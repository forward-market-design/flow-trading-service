-- Every so-often, a batch is compiled from the current submissions.
-- An auction is then run, asynchronously updating the *_outcomes fields with the outcomes.
create table batch (
    id integer primary key,
    as_of text not null,
    portfolio_outcomes blob not null, -- Json<Record<PortfolioId, PortfolioOutcome>>
    product_outcomes blob not null -- Json<Record<ProductId, ProductOutcome>>
) strict;
--
-- the output of the batch auction with respect to the portfolios
create table portfolio_outcome (
    portfolio_id text not null,
    trade real not null,
    price real,
    data blob,
    valid_from text not null,
    valid_until text,
    primary key (portfolio_id, valid_from),
    unique (valid_from, valid_until, portfolio_id),
    foreign key (portfolio_id) references portfolio (id)
) strict, without rowid;
--
-- the output of the batch auction with respect to the products
create table product_outcome (
    product_id text not null,
    trade real not null,
    price real,
    data blob,
    valid_from text not null,
    valid_until text,
    primary key (product_id, valid_from),
    unique (valid_from, valid_until, product_id),
    foreign key (product_id) references product (id)
) strict, without rowid;
--
-- as with the other tables, we use triggers to maintain the outputs
create trigger batch_update_portfolio_trigger
after update of portfolio_outcomes on batch
begin
-- invalidate existing output
update portfolio_outcome
set
    valid_until = new.as_of
where
    valid_from = old.as_of;
-- create new output
insert into portfolio_outcome (
    portfolio_id, trade, price, data, valid_from, valid_until
)
select
    "key",
    jsonb_extract(value, '$.trade') as trade,
    jsonb_extract(value, '$.price') as price,
    jsonb_extract(value, '$.data') as data,
    new.as_of, -- noqa: RF01
    null as valid_until
from
    json_each(new.portfolio_outcomes);
end;

create trigger batch_update_product_trigger
after update of product_outcomes on batch
begin
-- invalidate existing output
update
product_outcome
set
    valid_until = new.as_of
where
    valid_from = old.as_of;
-- create new output
insert into
product_outcome (product_id, trade, price, data, valid_from, valid_until)
select
    "key",
    jsonb_extract(value, '$.trade') as trade,
    jsonb_extract(value, '$.price') as price,
    jsonb_extract(value, '$.data') as data,
    new.as_of, -- noqa: RF01
    null as valid_until
from
    json_each(new.product_outcomes);
end;
