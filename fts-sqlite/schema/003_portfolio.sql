create table portfolio (
    id text primary key,
    as_of text not null,
    bidder_id text not null,
    app_data blob not null, -- Json<Any>
    demand_group blob, -- Option<Json<Record<DemandId, number>>>
    product_group blob -- Option<Json<Record<ProductId, number>>>
) strict, without rowid;
--
create index portfolio_by_bidder on portfolio (bidder_id, id);
--
-- we track the lifetimes of the portfolio's demand group
create table demand_group (
    portfolio_id text not null,
    demand_id text not null,
    weight real not null,
    valid_from text not null,
    valid_until text,
    -- query by portfolio or by demand equally efficiently
    primary key (portfolio_id, demand_id, valid_from),
    unique (demand_id, portfolio_id, valid_from),
    -- build an index for efficient batch auction generation
    unique (valid_until, valid_from, portfolio_id, demand_id),
    foreign key (portfolio_id) references portfolio (id),
    foreign key (demand_id) references demand (id)
) strict, without rowid;
--
-- we track the lifetimes of the portfolio's product group
create table product_group (
    portfolio_id text not null,
    product_id text not null,
    weight real not null,
    valid_from text not null,
    valid_until text,
    primary key (portfolio_id, product_id, valid_from),
    unique (valid_until, valid_from, portfolio_id, product_id),
    foreign key (portfolio_id) references portfolio (id),
    foreign key (product_id) references product (id)
) strict, without rowid;
--
-- we use triggers to automatically maintain these lifetime tables
--
--  when a new portfolio is created, insert new lifetime records in both tables
create trigger portfolio_insert_trigger
after insert on portfolio
begin
-- track the demand lifetime
insert into demand_group (
    portfolio_id, demand_id, weight, valid_from, valid_until
)
select
    new.id, -- noqa: RF01
    key,
    value,
    new.as_of,
    null
from
    json_each(new.demand_group);
    -- track the product lifetime
insert into
product_group (portfolio_id, product_id, weight, valid_from, valid_until)
select
    new.id, -- noqa: RF01
    key,
    value,
    new.as_of,
    null
from
    json_each(new.product_group);
end;
-- when an existing portfolio has its demand_group updated:
create trigger portfolio_update_demand_group_trigger
after update of demand_group on portfolio
begin
update demand_group
set
    valid_until = new.as_of
where
    portfolio_id = old.id
    and
    valid_from <= old.as_of
    and
    valid_until is null;
insert into demand_group (
    portfolio_id,
    demand_id,
    weight,
    valid_from,
    valid_until
)
select
    new.id, -- noqa: RF01
    key,
    value,
    new.as_of,
    null
from
    json_each(new.demand_group);
end;
-- when an existing portfolio has its product_group updated:
create trigger portfolio_update_product_group_trigger
after update of product_group on portfolio
begin
update product_group
set
    valid_until = new.as_of
where
    portfolio_id = old.id
    and
    valid_from <= old.as_of
    and
    valid_until is null;
insert into product_group (
    portfolio_id,
    product_id,
    weight,
    valid_from,
    valid_until
)
select
    new.id, -- noqa: RF01
    key,
    value,
    new.as_of,
    null
from
    json_each(new.product_group);
end;
