# Flow Trading Service (FTS)

This project represents a complete implementation of *flow trading* as proposed
by [Budish, Cramton, et al](https://cramton.umd.edu/papers2020-2024/budish-cramton-kyle-lee-malec-flow-trading.pdf),
in which trade occurs continuously over time via regularly-scheduled batch auctions. We define a core set of primitives in `fts-core` (so-called
"models" and "ports", using the terminology of hexagonal architecture), a
reference solver for the relevant quadratic program in `fts-solver`, a REST API HTTP server for interacting with the solver in `fts-server`, and
finally an implementation of the core data operations in `fts-demo` using
SQLite, suitable for exploration of flow trading-based marketplaces such as a forward market.

These 4 crates each contain their own `README.md` which explains the crate's functionality and the relevant high-level concepts. We explicitly call out `fts-core/README.md` as an introduction to the bid primitives used in our flow trading implementation.

## Quick Start

To get started, ensure both CMake and Rust >= 1.85 are available in your system `PATH`. Refer to [Rustup](https://rustup.rs/) for an easy way to install Rust. Installing CMake can be done via your system's packaging utility, e.g. `brew install cmake` or `apt-get install cmake`. Then, paste the following into your CLI:

```bash
# Clone the repository if necessary
git clone https://github.com/forward-market-design/flow-trading-service.git
cd flow-trading-service

# Compile and run the demonstration server
cargo run --release --bin fts-demo -- --api-secret SECRET --trade-rate 1h
```

This will download the project dependencies, build the software, and then run the server on port 8080. The OpenAPI specification will be available at http://localhost:8080/rapidoc if the server is successfully running. A Dockerfile is also available to build and run the binary.

Refer to [`fts-demo/README.md`](./fts-demo/README.md) for full documentation of the available configuration options and their meaning.

## Example

Interacting with the server is intended to occur through an API client, such as the one being developed in [flow-trading-client](https://github.com/forward-market-design/flow-trading-client). However, the server is nothing more than REST endpoints operating on HTTP requests. Accordingly, we provide an example of a hypothetical trading session below that can be copied and pasted into any command-line environment with `curl`, `date`, and `jq` commands available. (Note that Windows users will need to replace the line continuations `\` with `^` for the example to work.)

Supposing `fts-demo` is running as above, with the signing secret set to `SECRET` and running on `localhost:8080`, the following commands exercise some basic functionality.

First, we define a few JWT tokens for use by the bidders and admin. Authenticated requests include a JWT token with the `sub:` claim set to the bidder id. Admin actions require an additional custom claim `admin: true`.
```bash
# Signed JWT for user 11111111-1111-8111-8111-111111111111
JWTB1=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMTExMTExMS0xMTExLTgxMTEtODExMS0xMTExMTExMTExMTEifQ.jIy_I8E-VW1ToODyVzqU6dLrLaXKnbFGDvbqTs4N-Jo
# Signed JWT for user 22222222-2222-8222-8222-222222222222
JWTB2=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIyMjIyMjIyMi0yMjIyLTgyMjItODIyMi0yMjIyMjIyMjIyMjIifQ.4x0mSmuS9s9CnMOYhjTd8WPB2ZUo0P3V0ak5Mdc0W1c
# Signed JWT for an admin user
JWTADMIN=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJGRkZGRkZGRi1GRkZGLUZGRkYtRkZGRi1GRkZGRkZGRkZGRkYiLCJhZG1pbiI6dHJ1ZX0.CV3aldMRLqaRY2UsKYxFpC-tWI8EbJoXl7YlF4gYcjY
```

Next, we need to define some products to trade. `fts-demo` implements a basic product suitable for forward markets, each defined by the triplet `(kind, from, thru)`. `kind` is an arbitrary string that classifies the product, while `from` and `thru` refer to the delivery window of the product. This endpoint will return the system-generated ids for each product.
```bash
PRODUCT=$(\
  curl -s -X POST -H "Content-Type: application/json" -H "Authorization: Bearer $JWTADMIN" \
    --data '[{ "kind": "DEMO", "from": "2030-01-01T00:00:00Z", "thru": "2030-01-01T06:00:00Z" }]' \
    http://localhost:8080/admin/products \
  | jq -r '.[0]' \
)
```

Now we define a submission for the first bidder. Refer to `fts-core/README.md` to learn more about submissions and how they work. Notably, when defining `auths` and `costs`, client-generated identifiers are allowed if they do not generate any collisions.
```bash
curl -s -X PUT -H "Content-Type: application/json" -H "Authorization: Bearer $JWTB1" \
  --data '{
    "auths": [{
      "auth_id": "11111111-AAAA-8111-8111-111111111111",
      "portfolio": { "'"$PRODUCT"'": 1 },
      "data": {}
    }],
    "costs": [{
      "cost_id": "11111111-CCCC-8111-8111-111111111111",
      "group": { "11111111-AAAA-8111-8111-111111111111": 1 },
      "data": [{ "rate": 0, "price": 15 }, { "rate": 20, "price": 5 }]
    }]
  }' http://localhost:8080/v0/submissions/11111111-1111-8111-8111-111111111111 | jq . 
```

The first bidder expressed downward-sloping demand for the product. Suppose the second bidder is a supplier with fixed marginal cost and unlimited supply. Supply is represented by negative rates and demand by positive rates.
```bash
curl -s -X PUT -H "Content-Type: application/json" -H "Authorization: Bearer $JWTB2" \
   --data '{
    "auths": [{
      "auth_id": "22222222-AAAA-8222-8222-222222222222",
      "portfolio": { "'"$PRODUCT"'": 1 },
      "data": {}
    }],
    "costs": [{
        "cost_id": "22222222-CCCC-8222-8222-222222222222",
        "group": { "22222222-AAAA-8222-8222-222222222222": 1 },
        "data": { "max_rate": 0, "price": 10 }
    }]
  }' http://localhost:8080/v0/submissions/22222222-2222-8222-8222-222222222222 | jq . 
```

Auctions are explicitly triggered by an administrative action. We run a single auction spanning the next hour:
```bash
curl -s -i -X POST -H "Content-Type: application/json" -H "Authorization: Bearer $JWTADMIN" \
  --data '{ "by": "1h", "thru": "'"$(date --date="+1 hour" --rfc-3339=seconds)"'" }' \
  http://localhost:8080/admin/auctions/solve
```

Finally, one is likely concerned with how their submissions are doing and/or how individual products are trading:
```bash
# Bidder 1's outcomes for each auction 
curl -s -H "Authorization: Bearer $JWTB1" http://localhost:8080/v0/auths/11111111-AAAA-8111-8111-111111111111/outcomes | jq .

# Bidder 2's outcomes for each auction
curl -s -H "Authorization: Bearer $JWTB2" http://localhost:8080/v0/auths/22222222-AAAA-8222-8222-222222222222/outcomes | jq .

# The demonstration product's outcomes for each auction
curl -s http://localhost:8080/v0/products/$PRODUCT/outcomes | jq .
```

Please refer to the various crate README.md files for discussion on the more advanced functionality available.
