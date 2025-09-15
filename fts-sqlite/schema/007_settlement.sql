
-- Note that the definition of the `settlement` table awkwardly occurs in batch.sql,
-- but SQLite doesn't allow foreign key constraints inside alter table statements.
--
-- We use a trigger to automatically aggregate batches to a new settlement.
create trigger settlement_insert_trigger
after insert on settlement
begin
update
    batch
set
    settled = new.as_of
where
    settled is null
and
    valid_until <= new.as_of;
end;
--
-- We also have tables to store the rounded (with respect to settlement decimals) outcomes
create table settlement_position (
    as_of text not null,
    bidder_id text not null,
    product_id text not null,
    position integer not null,
    primary key (as_of, bidder_id, product_id),
    foreign key (as_of) references settlement (as_of),
    foreign key (product_id) references product (id)
) strict, without rowid;
--
create table settlement_payment (
    as_of text not null,
    bidder_id text not null,
    payment integer not null,
    primary key (as_of, bidder_id),
    foreign key (as_of) references settlement (as_of)
) strict, without rowid;
--
-- We additionally create triggers for the UPDATE so we can insert all the
-- positions and payments in a simple way from the Rust code.
--
create trigger settlement_update_positions_trigger
after update of positions on settlement
begin
    insert into settlement_position (
        as_of,
        bidder_id,
        product_id,
        position
    )
    select
        new.as_of,
        jsonb_extract(data.value, '$.bidder_id'),
        jsonb_extract(data.value, '$.product_id'),
        jsonb_extract(data.value, '$.rounded')
    from
        json_each(new.positions) as data;
end;
--
create trigger settlement_update_payments_trigger
after update of payments on settlement
begin
    insert into settlement_payment (
        as_of,
        bidder_id,
        payment
    )
    select
        new.as_of,
        data.key,
        data.atom
    from
        json_each(new.payments) as data;
end;