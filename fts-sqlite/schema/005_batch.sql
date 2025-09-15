-- Every so-often, a settlement is compiled from the current batches.
-- This is declared here so we can establish the foreign key reference,
-- but the remainder of the settlement logic will be in its own .sql file.
create table settlement (
    as_of text primary key,
    position_decimals integer not null,
    payment_decimals integer not null,
    positions blob,
    payments blob
) strict, without rowid;
--
-- Every so-often, a batch is compiled from the current submissions.
-- An auction is then run, asynchronously updating the *_outcomes fields with the outcomes.
create table batch (
    id integer primary key,
    valid_from text not null,
    valid_until text,
    portfolio_outcomes blob not null, -- Json<Record<PortfolioId, Outcome<PortfolioOutcome>>>
    product_outcomes blob not null, -- Json<Record<ProductId, Outcome<ProductOutcome>>>
    settled text,
    time_unit_in_secs real not null,
    foreign key (settled) references settlement (as_of)
) strict;
--
-- the output of the batch auction with respect to the portfolios
create table batch_portfolio (
    batch_id integer not null,
    portfolio_id text not null,
    trade real not null,
    price real,
    data blob,
    primary key (batch_id, portfolio_id),
    foreign key (batch_id) references batch (id),
    foreign key (portfolio_id) references portfolio (id)
) strict, without rowid;
--
-- the output of the batch auction with respect to the products
create table batch_product (
    batch_id integer not null,
    product_id text not null,
    trade real not null,
    price real,
    data blob,
    primary key (batch_id, product_id),
    foreign key (batch_id) references batch (id),
    foreign key (product_id) references product (id)
) strict, without rowid;
--
-- as with the other tables, we use triggers to maintain the outputs
create trigger batch_insert_trigger
after insert on batch
begin
-- 1. Terminate "old" batches by setting old.valid_until = new.valid_from.
update batch
set
    valid_until = new.valid_from
where
    valid_from < new.valid_from
    and
    valid_until is null;
-- 2. Destructure and propagate the portfolio outcomes
insert into batch_portfolio (
    batch_id, portfolio_id, trade, price, data
)
select
    new.id,
    "key",
    jsonb_extract(value, '$.trade') as trade,
    jsonb_extract(value, '$.price') as price,
    jsonb_extract(value, '$.data') as data
from
    json_each(new.portfolio_outcomes);
-- 3. Destructure and propagate the product outcomes
insert into batch_product (
    batch_id, product_id, trade, price, data
)
select
    new.id,
    "key",
    jsonb_extract(value, '$.trade') as trade,
    jsonb_extract(value, '$.price') as price,
    jsonb_extract(value, '$.data') as data
from
    json_each(new.product_outcomes);
end;
