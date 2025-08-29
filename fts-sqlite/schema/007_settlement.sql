
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
    settlement_id = new.id
where
    settlement_id is null
and
    valid_until <= new.as_of;
end;
--
-- We also have tables to store the rounded (with respect to settlement decimals) outcomes
create table settlement_position (
    settlement_id integer not null,
    bidder_id text not null,
    product_id text not null,
    position integer not null,
    primary key (settlement_id, bidder_id, product_id),
    foreign key (settlement_id) references settlement (id),
    foreign key (product_id) references product (id)
) strict, without rowid;
--
create table settlement_payment (
    settlement_id integer not null,
    bidder_id text not null,
    payment integer not null,
    primary key (settlement_id, bidder_id),
    foreign key (settlement_id) references settlement (id)
) strict, without rowid;
