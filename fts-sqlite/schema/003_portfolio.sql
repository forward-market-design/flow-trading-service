create table portfolio (
    id text primary key,
    as_of text not null,
    bidder_id text not null,
    app_data blob not null, -- Json<Any>
    demand blob, -- Option<Json<Record<DemandId, number>>>
    basis blob -- Option<Json<Record<ProductId, number>>>
) strict, without rowid;
--
create index portfolio_by_bidder on portfolio (bidder_id, id);
--
-- we track the lifetimes of the portfolio's demand group
create table portfolio_demand (
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
create table portfolio_product (
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
insert into portfolio_demand (
    portfolio_id, demand_id, weight, valid_from, valid_until
)
select
    new.id, -- noqa: RF01
    key,
    value,
    new.as_of,
    null
from
    json_each(new.demand);
    -- track the product lifetime
insert into
portfolio_product (portfolio_id, product_id, weight, valid_from, valid_until)
select
    new.id, -- noqa: RF01
    key,
    value,
    new.as_of,
    null
from
    json_each(new.basis);
end;
-- when an existing portfolio has its demand updated:
create trigger portfolio_update_demand_trigger
after update of demand on portfolio
begin
update portfolio_demand
set
    valid_until = new.as_of
where
    portfolio_id = old.id
    and
    valid_from <= old.as_of
    and
    valid_until is null;
insert into portfolio_demand (
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
    json_each(new.demand);
end;
-- when an existing portfolio has its basis updated:
create trigger portfolio_update_basis_trigger
after update of basis on portfolio
begin
update portfolio_product
set
    valid_until = new.as_of
where
    portfolio_id = old.id
    and
    valid_from <= old.as_of
    and
    valid_until is null;
insert into portfolio_product (
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
    json_each(new.basis);
end;
