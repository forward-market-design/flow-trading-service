create table demand (
    id text primary key,
    as_of text not null,
    bidder_id text not null,
    app_data blob not null, -- Json<Any>
    curve_data blob -- Option<Json<Demand>>
) strict, without rowid;
--
create index demand_by_bidder on demand (bidder_id, id);
--
-- manage updates to demand curve data
create table curve_data (
    -- the demand curve to attach this data to
    demand_id text not null,
    -- a jsonb representation of the demand curve
    value blob,
    -- what is the lifetime of this change?
    valid_from text not null,
    valid_until text,
    -- create an index (and eliminate race conditions) for querying
    primary key (demand_id, valid_from),
    unique (valid_from, valid_until, demand_id),
    foreign key (demand_id) references demand (id)
) strict, without rowid;
--
-- These triggers maintain the demand_value table:
--  1. When a new demand is created:
create trigger demand_insert_trigger
after insert on demand
begin
insert into curve_data (
    demand_id,
    value,
    valid_from,
    valid_until
)
values (
    new.id,
    new.curve_data,
    new.as_of,
    null
);
end;
-- 2. When an existing demand is updated:
create trigger demand_update_trigger
after update on demand
begin
update curve_data
set
    valid_until = new.as_of
where
    demand_id = old.id
    and
    valid_from = old.as_of;
insert into curve_data (
    demand_id, value, valid_from, valid_until
)
values (
    new.id, new.curve_data, new.as_of, null
);
end
