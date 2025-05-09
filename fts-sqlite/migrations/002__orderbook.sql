-- Once running, we should preserve the configuration that has been applied to
-- stored data.
create table config (
    id integer primary key check (id = 0),
    data text not null check (json_valid(data))
) strict;
--
-- A auth is a direction in product space
create table auth (
    id blob primary key,
    bidder_id blob not null
) strict,
without rowid;
--
-- A auth_weight associates a (product, weight) to an auth. We make no attempt
-- to "canonicalize" an order (non-zero weight, unique (auth_id, product_id))
-- because we expect that, in the presence of a product hierarchy, when we project
-- down onto the leaf nodes there may be duplicate entries anyway.
create table auth_weight (
    auth_id blob not null,
    product_id blob not null,
    weight real not null,
    primary key (auth_id, product_id),
    foreign key (auth_id) references auth (id),
    foreign key (product_id) references product (id)
) strict,
without rowid;
--
-- An auth_data is an explicit grant of permission to trade a collection of
-- products. To remove an auth_data, create a new one with content = null.
create table auth_data (
    id integer primary key,
    auth_id blob not null,
    version text not null,
    content text check (json_valid(content)),
    -- this simple constraint restricts the auth_data updates to unique times.
    -- we presently track timestamps at the nanosecond resolution, so if somebody
    -- updates the same auth twice in the same nanosecond, an error will occur.
    -- This is fine.
    unique (auth_id, version),
    foreign key (auth_id) references auth (id)
) strict;
--
-- A cost is a collection of substitutable auths and a demand curve
create table cost (
    id blob primary key,
    bidder_id blob not null
) strict,
without rowid;
--
-- A cost_weight associates a (auth, weight) to a cost.
create table cost_weight (
    cost_id blob not null,
    auth_id blob not null,
    weight real not null,
    primary key (cost_id, auth_id),
    foreign key (cost_id) references cost (id),
    foreign key (auth_id) references auth (id),
    unique (auth_id, cost_id)
) strict,
without rowid;
--
-- The version demand curves for the cost
create table cost_data (
    id integer primary key,
    cost_id blob not null,
    version text not null,
    content text check (json_valid(content)),
    -- same observation here about the nanosecond resolution
    unique (cost_id, version),
    foreign key (cost_id) references cost (id)
) strict;
--
-- Submissions are processed in auctions. We will record execution of submissions in these auctions
create table auction (
    id integer primary key,
    "from" text not null unique,
    "thru" text not null unique,
    -- These are timestamps
    queued text not null,
    solved text,
    -- We store a JSON record of the processed auction input for later use
    auction text
) strict;
--
-- The result of an auth in an auction
create table auth_outcome (
    auction_id integer not null,
    auth_id blob not null,
    price real not null,
    trade real not null,
    primary key (auction_id, auth_id),
    foreign key (auction_id) references auction (id),
    foreign key (auth_id) references auth (id),
    -- this creates an index over auth_id, allowing us to efficiently find running totals
    unique (auth_id, auction_id)
) strict,
without rowid;
--
-- All products get periodic updates as a result of auction execution.
-- In the very least, this data consists of the timestamp, the clearing price,
-- and the trade volume.
-- Additional data, e.g. marginalized supply/demand curves, can be added later
-- via migration.
create table product_outcome (
    auction_id integer not null,
    product_id blob not null,
    price real not null,
    trade real not null,
    primary key (auction_id, product_id),
    unique (product_id, auction_id),
    foreign key (product_id) references product (id)
) strict,
without rowid;
