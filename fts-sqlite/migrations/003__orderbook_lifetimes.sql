-- Given the current schema, there is not a performant way to query the orders
-- that belong to a specific auction. Fundamentally, this is because we do not
-- store the "retirement" time for an order, only its creation time. We are
-- going to create a table that tracks and maintains this information via
-- triggers so the client does not need to be responsible for it.
create table auth_data_lifetime (
    id integer primary key,
    birth text not null,
    death text,
    foreign key (id) references auth_data (id)
) strict,
without rowid;
create table cost_data_lifetime (
    id integer primary key,
    birth text not null,
    death text,
    foreign key (id) references cost_data (id)
) strict,
without rowid;

--
-- To make queries efficient, we build some indices. TODO (measure!)
create index auth_data_lifetime_index on auth_data_lifetime (death, birth);
create index cost_data_lifetime_index on cost_data_lifetime (death, birth);
--
-- Here are the triggers
--
create trigger auth_data_lifetime_trigger
after
insert on auth_data begin
-- set the death time for the old auth_data
update auth_data_lifetime
set death = new.version
from (
    select id
    from auth_data
    where
        auth_id = new.auth_id --noqa: RF01
        and version < new.version --noqa: RF01
    order by version desc
    limit 1
) as prev
where auth_data_lifetime.id = prev.id;
-- set the birth time for the new auth_data
insert into auth_data_lifetime (id, birth)
values (new.id, new.version);
end;
--
create trigger cost_data_lifetime_trigger
after
insert on cost_data begin
-- set the death time for the old cost_data
update cost_data_lifetime
set death = new.version
from (
    select id
    from cost_data
    where
        cost_id = new.cost_id --noqa: RF01
        and version < new.version --noqa: RF01
    order by version desc
    limit 1
) as prev
where cost_data_lifetime.id = prev.id;
-- set the birth time for the new cost_data
insert into cost_data_lifetime (id, birth)
values (new.id, new.version);
end;
